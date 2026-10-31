"""The pole-drop check: does MuJoCo's hinge friction match the real pole?

On the rig, the pole let go from horizontal with the cart held still swings
back and forth until friction stops it. This drops the pole the same way in
MuJoCo, faster than real time, and counts the swings, once for each hinge
frictionloss in trainer/pole_drop.toml, so the value can be tuned until
MuJoCo's count matches the rig's.

It runs the model the loop runs: the MJCF and joint names from
trainer/settings.toml ([orchestrator] cwd, [mjcf]), and the mujoco.dll the
orchestrator links (orchestrator.toml mujoco_dir), called through ctypes,
so nothing is installed. For each value it writes a copy of the MJCF beside
the original (so the meshes' relative paths still resolve) with only the
pole joint's frictionloss changed, and deletes it after.

The cart is frozen at the middle (user, 2026-10-29): after every step its
place and speed are set back, as the rig's powered stepper holds it, so the
hinge's friction is all that slows the pole. Left to the model's velocity
servo, commanded to 0 m/s, the swinging pole pushed the cart about 4 mm
either way, and that wobble took most of the pole's energy.

Counting: a swing is one pass of the pole through hanging (its angle
changing sign), the way the rig's swings were counted (user, 2026-10-29: 19
in MuJoCo's simulate viewer at frictionloss 0.02, where this counts 18). A
swing's peak is the largest angle between two passes.

usage (from the repo root):
  uv run --project trainer python trainer/src/pole_drop.py
"""

import ctypes
import math
import os
import re
import sys
import tomllib
from pathlib import Path

TRAINER = Path(__file__).resolve().parent.parent
REPO = TRAINER.parent

# mjtype.h (as physics/src/ffi.rs mirrors them, MuJoCo 3.15.0).
mjOBJ_JOINT = 3
mjSTATE_TIME = 1 << 0
mjSTATE_QPOS = 1 << 1
mjSTATE_QVEL = 1 << 2


def load_toml(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def mujoco() -> ctypes.CDLL:
    """The mujoco.dll orchestrator.toml names, with the signatures used here."""
    bin_dir = (REPO / load_toml(REPO / "orchestrator.toml")["mujoco_dir"] / "bin").resolve()
    os.add_dll_directory(str(bin_dir))
    lib = ctypes.CDLL(str(bin_dir / "mujoco.dll"))
    p, i = ctypes.c_void_p, ctypes.c_int
    lib.mj_loadXML.argtypes, lib.mj_loadXML.restype = [ctypes.c_char_p, p, ctypes.c_char_p, i], p
    lib.mj_makeData.argtypes, lib.mj_makeData.restype = [p], p
    lib.mj_deleteModel.argtypes, lib.mj_deleteData.argtypes = [p], [p]
    lib.mj_name2id.argtypes, lib.mj_name2id.restype = [p, i, ctypes.c_char_p], i
    lib.mj_stateSize.argtypes, lib.mj_stateSize.restype = [p, i], i
    lib.mj_getState.argtypes = [p, p, ctypes.POINTER(ctypes.c_double), i]
    lib.mj_setState.argtypes = [p, p, ctypes.POINTER(ctypes.c_double), i]
    lib.mj_step.argtypes = lib.mj_forward.argtypes = [p, p]
    return lib


def with_frictionloss(xml: str, joint: str, value: float) -> str:
    """The MJCF text with `joint`'s frictionloss set to `value` and nothing
    else changed."""
    pattern = re.compile(r'(<joint\b[^>]*\bname="' + re.escape(joint) + r'"[^>]*\bfrictionloss=")[^"]*(")')
    changed, n = pattern.subn(lambda m: f"{m.group(1)}{value!r}{m.group(2)}", xml)
    if n != 1:
        raise SystemExit(f"pole_drop: found {n} joints named {joint!r} with a frictionloss in the MJCF, not 1")
    return changed


def drop(lib, model_path: Path, pole_joint: str, cart_joint: str, settings: dict) -> dict:
    """One drop: the pole let go from start_degrees, counted until it stops."""
    error = ctypes.create_string_buffer(1000)
    m = lib.mj_loadXML(str(model_path).encode(), None, error, len(error))
    if not m:
        raise SystemExit(f"pole_drop: MuJoCo could not load {model_path}: {error.value.decode()}")
    d = lib.mj_makeData(m)
    try:
        pole = lib.mj_name2id(m, mjOBJ_JOINT, pole_joint.encode())
        cart = lib.mj_name2id(m, mjOBJ_JOINT, cart_joint.encode())
        # [time, qpos x 2, qvel x 2]; a 1-DOF joint's qpos and qvel index is
        # its id (physics P2).
        sig = mjSTATE_TIME | mjSTATE_QPOS | mjSTATE_QVEL
        size = lib.mj_stateSize(m, sig)
        if pole < 0 or cart < 0 or size != 5:
            raise SystemExit(f"pole_drop: expected joints {pole_joint!r} and {cart_joint!r}, two qpos and two "
                             f"qvel, got ids {pole} and {cart}, state {size}")
        state = (ctypes.c_double * size)()
        lib.mj_getState(m, d, state, sig)
        state[1 + pole] = math.radians(settings["start_degrees"])
        lib.mj_setState(m, d, state, sig)
        lib.mj_forward(m, d)
        home = state[1 + cart]

        passes, peak, last_pass = 0, 0.0, 0.0
        before = state[1 + pole]
        marks = sorted(settings["marks_degrees"], reverse=True)
        reached = {}
        stop = math.radians(settings["stop_degrees"])
        while True:
            lib.mj_step(m, d)
            lib.mj_getState(m, d, state, sig)
            state[1 + cart], state[3 + cart] = home, 0.0
            lib.mj_setState(m, d, state, sig)
            t, angle = state[0], state[1 + pole]
            peak = max(peak, abs(angle))
            if (angle < 0) != (before < 0):
                passes += 1
                last_pass = t
                for mark in marks:
                    if mark not in reached and peak < math.radians(mark):
                        reached[mark] = passes
                if peak < stop:
                    why = f"a swing peaked under {settings['stop_degrees']} deg"
                    break
                peak = 0.0
            before = angle
            if t - last_pass > settings["still_seconds"]:
                why = f"no pass for {settings['still_seconds']} s, resting at {math.degrees(angle):.2f} deg"
                break
            if t > settings["max_seconds"]:
                why = f"not stopped after {settings['max_seconds']} s"
                break
        return {"passes": passes, "time": last_pass, "why": why, "reached": reached}
    finally:
        lib.mj_deleteData(d)
        lib.mj_deleteModel(m)


def main() -> int:
    settings = load_toml(TRAINER / "pole_drop.toml")
    run = load_toml(TRAINER / "settings.toml")
    model = (TRAINER / run["orchestrator"]["cwd"] / run["mjcf"]["model"]).resolve()
    pole_joint, cart_joint = run["mjcf"]["pole_joint"], run["mjcf"]["cart_joint"]
    lib = mujoco()
    xml = model.read_text(encoding="utf-8")
    copy = model.with_name(model.stem + ".pole-drop.xml")
    print(f"{model.name}: pole let go from {settings['start_degrees']} deg, cart frozen at the middle")
    try:
        for value in settings["frictionloss"]:
            copy.write_text(with_frictionloss(xml, pole_joint, value), encoding="utf-8")
            r = drop(lib, copy, pole_joint, cart_joint, settings)
            marks = ", ".join(f"<{mark:g} deg after {r['reached'][mark]}" for mark in sorted(r["reached"], reverse=True))
            print(f"frictionloss {value:g}: {r['passes']} swings in {r['time']:.1f} s simulated; "
                  f"stopped: {r['why']}")
            if marks:
                print(f"    swings until one first peaks {marks}")
    finally:
        copy.unlink(missing_ok=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
