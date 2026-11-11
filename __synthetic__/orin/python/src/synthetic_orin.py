"""The synthetic Orin: the robot computer's stand-in in the simulated loop.

The policy has to learn from what the robot will see and remember, so this
process does what the real Orin does with each camera frame - detect the
four markers, keep the last six detections as the policy's timestamped
window (synthetic-orin-plan.md). Its camera is the Unity player: it serves
the frame socket Unity connects to, triggers each frame there with the
pose the orchestrator sends, and gets the JPEG back (__documentation__/synthetic-loop-dia.md,
4-14). It writes each frame to <out>/frame_NNNNNN.jpg and one observation
line per frame to <out>/observations.jsonl, and replies to each pose with
the pool (14). The policy has to know where things are in metres, so Pivot's
and End's pixels go through the matrix the calibration found before they
enter the pool (__documentation__/synthetic-loop-dia.md, Calibration).

It runs in one of two ways, each with the same start and end:
- --calibrate FILE: finds the matrix and writes it to FILE (calibration.py).
  The orchestrator runs one of these before the training instances start.
- --homography FILE --pool-gaps d1,...,d6: one training instance's Orin. It
  reads the matrix once here and holds it in memory for the whole run.

The start (__documentation__/synthetic-loop-dia.md): pre-warm the pool, open the frame
socket (the note at 4); connect to the orchestrator's control socket (5);
wait for Unity (7); tell the orchestrator {"type": "registered"} (8). The
end: the orchestrator closes the control socket (22), this process closes
the frame socket (23), Unity quits (24), and this process exits (25).

Messages, control socket (orchestrator):
- out text   {"type": "registered"}                  once, when Unity has
                                                     connected (8)
- in  text   {"id": n, "time": t, "joints": [...]}   the capture state (11)
- in  text   {"type": "reset", "gaps": [d1, ... d6]} between episodes (R5):
                                                     pre-warm afresh at them
- out text   {"type": "reset"}                       the pool is ready (R7)
- out text   {"id": n, "pool": [[age_ms, pivot_x, pivot_z, end_x, end_z] x6],
              "failed": bool, "state": {"cart", "cos_theta", "sin_theta",
              "cart_speed", "pole_speed"}}           exactly one per pose,
                                                     in order (14): the
                                                     pool newest first;
                                                     failed ends the run;
                                                     state the five numbers
                                                     (derived_state.py)
Messages, frame socket (Unity):
- out text   {"id": n, "joints": [...]}              the trigger (12): the
                                                     pose, forwarded as is
- in  binary the JPEG                                one per trigger (13)

Rules:
- Times are simulated time (O2): `time` is the capture time the
  orchestrator sends (MuJoCo's data.time), never a wall clock.
- The window holds the last 6 frames' points (capture time + Pivot's and
  End's (x, z) in metres, or (-1, -1) where not detected); the 7th append
  drops the oldest. Ages = capture time - newest capture time
  (newest 0, older negative).
- Pre-warm (__documentation__/synthetic-loop-dia.md, the note at 4; The initial pool): at
  launch, before any frame, the window is six entries of no detection,
  (-1, -1) x2, at times -d1, -(d1+d2), ..., -(d1+...+d6) before the first
  capture (t = 0). The gaps dk are the orchestrator's, --pool-gaps, from its
  schedule: the gaps between the captures of steps 0, -1, ..., -5, and d6 =
  d5, which never reaches the policy. This process picks nothing and holds
  no timing of its own, since the orchestrator alone decides when a capture
  happens. Every frame, the first
  included, is then one append: frame 0's ages are 0, -d1, ..., -(d1+...+d5).
  A reset pre-warms the pool the same way at its gaps (Reset in place), so
  an episode after a reset sees the pool a fresh launch with that seed
  would. Frame ids count on across the run's episodes, so frame files and
  log lines stay unique.
- A failed frame (__documentation__/synthetic-loop-dia.md, the note at 13; O3): Unity's
  JPEG does not decode or is not 728x544. Its entry is (-1, -1) x2, the
  error is logged, and it does not fail the run on its own; when 3 of the
  pool's 6 entries are failed frames, the reply says failed and the run
  fails. Pre-warm entries are no detection, not failed frames.
- --frames-listen is the frame socket's host:port; the orchestrator tells
  Unity the same address (6).

observations.jsonl, one line per frame:
  {"id", "time", "slots": [[x, y] x4], "confidences": [c x4], "detect_ms",
   "pool": [[age_ms, pivot_x, pivot_z, end_x, end_z] x6], "failed",
   "state": {"cart", "cos_theta", "sin_theta", "cart_speed", "pole_speed"}}
plus "error" for a failed frame, whose confidences and detect_ms are null.
`slots` are all four markers' detected positions, normalized to the frame;
`pool`, `failed` and `state` are exactly what the reply carries; `confidences`
and `detect_ms` are for checking. `state` is the five numbers DeepMind's
cart-pole policy reads, worked out from the pool (derived_state.py), each null
where a marker it needs was not seen. It reaches the orchestrator and stops
there: nothing of it reaches the policy yet. Nor do the first and last two.

usage (from the repo root):
  uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/synthetic_orin.py
      --orchestrator ws://... --frames-listen 127.0.0.1:<port>
      --out <dir> --engine <file> [--config <settings.toml>]
      (--calibrate <file> | --homography <file> --pool-gaps <d1,...,d6>)
"""

import argparse
import collections
import json
import math
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TextIO

from websockets.sync.client import ClientConnection, connect

import calibration
from derived_state import derive
from detector import Detector, FrameError
from detector_config import DEFAULT_PATH, DetectorConfig
from fit import Homography
from frame_socket import FrameSocket

# The loop's frame contract (__documentation__/synthetic-loop-dia.md, Contracts).
FRAME_SIZE = (728, 544)

# The policy input's window length (__documentation__/synthetic-loop-dia.md, Contracts).
WINDOW = 6

# A slot or point with no detection (__documentation__/synthetic-loop-dia.md, Contracts).
NOT_DETECTED = (-1.0, -1.0)

# What each pool entry holds: Pivot and End (__documentation__/synthetic-loop-dia.md, Contracts).
POINTS = 2

# Failed frames among the pool's entries that fail the run
# (__documentation__/synthetic-loop-dia.md, the note at 13).
FAILED_FRAMES_FAIL_RUN = 3


@dataclass(frozen=True)
class Entry:
    time: float  # capture time, simulated seconds
    points: list[tuple[float, float]]  # Pivot, End: (x, z) in metres, or (-1, -1)
    failed: bool = False  # Unity's frame broke the JPEG contract


def checked_gaps(gaps: object) -> list[float]:
    """Six finite gaps above 0 ms, from the orchestrator's schedule: an age must be
    strictly older than the one before it (__documentation__/synthetic-loop-dia.md, Contracts)."""
    if (not isinstance(gaps, list) or len(gaps) != WINDOW
            or not all(isinstance(g, (int, float)) and not isinstance(g, bool) and math.isfinite(g) and g > 0
                       for g in gaps)):
        raise ValueError(f"the pool's gaps are {WINDOW} finite numbers above 0 ms, got {gaps!r}")
    return [float(g) for g in gaps]


def pool_gaps(text: str) -> list[float]:
    """d1,...,d6, as --pool-gaps takes them."""
    try:
        return checked_gaps([float(part) for part in text.split(",")])
    except ValueError as error:
        raise argparse.ArgumentTypeError(str(error)) from None


def pre_warm(gaps_ms: list[float]) -> collections.deque[Entry]:
    """The pool before any frame: six no-detection entries before t = 0, at
    the given gaps, d1 first: the gap nearest t = 0."""
    times = []
    t = 0.0
    for gap in gaps_ms:
        t -= gap / 1000
        times.append(t)
    window: collections.deque[Entry] = collections.deque(maxlen=WINDOW)
    for t in reversed(times):  # the window runs oldest first
        window.append(Entry(t, [NOT_DETECTED] * POINTS))
    return window


def reset(message: dict) -> collections.deque[Entry]:
    """R6 (__documentation__/synthetic-loop-dia.md, Reset in place): the pool pre-warmed afresh
    at the reset's gaps, exactly as at launch, so no entry of the last
    episode survives and failed frames count afresh."""
    window = pre_warm(checked_gaps(message.get("gaps")))
    ages_ms = ", ".join(f"{entry.time * 1000:.3f}" for entry in reversed(window))
    print(f"synthetic orin: reset, pre-warmed, ages {ages_ms} ms", flush=True)
    return window


def listen_address(text: str) -> tuple[str, int]:
    """host:port, as --frames-listen takes it."""
    host, separator, port = text.rpartition(":")
    if not separator or not host or not port.isdigit() or not 1 <= int(port) <= 65535:
        raise argparse.ArgumentTypeError(f"expected host:port with a port 1-65535, got {text!r}")
    return host, int(port)


def pool_rows(window: collections.deque[Entry]) -> list[list[float]]:
    """The pool as the reply carries it: newest first, ages in ms from
    the newest capture (0, then negative), then pivot_x, pivot_z, end_x, end_z."""
    newest = window[-1].time
    return [
        [(entry.time - newest) * 1000, *(v for point in entry.points for v in point)]
        for entry in reversed(window)
    ]


def to_points(slots: list[tuple[float, float]], homography: Homography, pivot: int, end: int) -> list[tuple[float, float]]:
    """Pivot's and End's normalized slots as (x, z) in metres. A marker not detected
    stays (-1, -1): there is no pixel to map."""
    width, height = FRAME_SIZE
    return [
        NOT_DETECTED if slots[slot] == NOT_DETECTED
        else homography.to_metres(slots[slot][0] * width, slots[slot][1] * height)
        for slot in (pivot, end)
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--orchestrator", required=True, help="the orchestrator's WebSocket URL")
    parser.add_argument(
        "--frames-listen", type=listen_address, required=True, help="host:port of the frame socket Unity connects to"
    )
    parser.add_argument("--out", type=Path, required=True, help="the run's output folder")
    parser.add_argument(
        "--pool-gaps", type=pool_gaps, help="d1,...,d6: the pool's pre-warm gaps, ms, the orchestrator's"
    )
    parser.add_argument("--homography", type=Path, help="the matrix the calibration wrote: run as a training instance")
    parser.add_argument("--calibrate", type=Path, help="find the matrix and write it to this file, then exit")
    parser.add_argument(
        "--config", type=Path, default=DEFAULT_PATH, help="the detector's settings (default: __synthetic__/orin/python/settings.toml)"
    )
    parser.add_argument("--engine", type=Path, required=True, help="TensorRT engine built by build_engine.py")
    args = parser.parse_args()
    if bool(args.calibrate) == bool(args.homography and args.pool_gaps):
        parser.error("give --calibrate, or --homography with --pool-gaps")

    config = DetectorConfig.load(args.config)
    detector = Detector(config, args.engine, FRAME_SIZE)
    host, port = args.frames_listen
    frames = FrameSocket(host, port)
    print(f"synthetic orin: frame socket listening on ws://{host}:{port}", flush=True)
    try:
        if args.calibrate:
            with connect(args.orchestrator, max_size=None) as ws:
                print(f"synthetic orin: connected to {args.orchestrator}, calibrating", flush=True)
                camera = frames.wait_for_camera()
                print("synthetic orin: Unity connected to the frame socket", flush=True)
                ws.send(json.dumps({"type": "registered"}))
                calibration.run(detector, config, ws, camera, args.config, FRAME_SIZE, args.calibrate)
            return
        homography = Homography.load(args.homography, FRAME_SIZE)
        print(f"synthetic orin: the matrix from {args.homography}, lens number {homography.lens_k:+.3f}", flush=True)
        slots = (config.classes.index("Pivot"), config.classes.index("End"))
        window = pre_warm(args.pool_gaps)
        # Before frame 0, the entries' times are their ages from t = 0.
        ages_ms = ", ".join(f"{entry.time * 1000:.3f}" for entry in reversed(window))
        print(f"synthetic orin: pre-warmed, ages {ages_ms} ms", flush=True)
        log_path = args.out / "observations.jsonl"
        # Line-buffered: a run that fails midway leaves every finished line.
        with open(log_path, "w", buffering=1) as log, connect(args.orchestrator, max_size=None) as ws:
            print(f"synthetic orin: connected to {args.orchestrator}, logging {log_path}", flush=True)
            camera = frames.wait_for_camera()
            print("synthetic orin: Unity connected to the frame socket", flush=True)
            ws.send(json.dumps({"type": "registered"}))
            for message in ws:
                if isinstance(message, bytes):
                    raise RuntimeError("binary message on the control socket, expected a pose")
                pose = json.loads(message)
                if pose.get("type") == "reset":
                    window = reset(pose)
                    ws.send(json.dumps({"type": "reset"}))
                    continue
                frame_id, capture = int(pose["id"]), float(pose["time"])
                camera.send(json.dumps({"id": frame_id, "joints": pose["joints"]}))
                jpeg = camera.recv()
                if not isinstance(jpeg, bytes):
                    raise RuntimeError(f"frame {frame_id}: Unity sent text, expected the JPEG")
                (args.out / f"frame_{frame_id:06d}.jpg").write_bytes(jpeg)
                observe(detector, homography, slots, window, log, ws, frame_id, capture, jpeg)
    finally:
        frames.close()
        detector.close()


def observe(
    detector: Detector,
    homography: Homography,
    slots: tuple[int, int],
    window: collections.deque[Entry],
    log: TextIO,
    ws: ClientConnection,
    frame_id: int,
    capture: float,
    jpeg: bytes,
) -> None:
    """Detects, maps Pivot and End to metres, appends to the window, works out
    the five numbers, logs the line, and replies (14)."""
    started = time.perf_counter()
    observation: dict = {"id": frame_id, "time": capture}
    try:
        marker_frame = detector.detect(jpeg)
    except FrameError as error:
        # A failed frame: no detection, the error logged (the note at 13).
        window.append(Entry(capture, [NOT_DETECTED] * POINTS, failed=True))
        observation |= {
            "slots": [list(NOT_DETECTED)] * 4,
            "confidences": None,
            "detect_ms": None,
            "error": str(error),
        }
        print(f"synthetic orin: frame {frame_id} failed: {error}", flush=True)
    else:
        detect_ms = (time.perf_counter() - started) * 1000
        window.append(Entry(capture, to_points(marker_frame.slots, homography, *slots)))
        observation |= {
            "slots": [list(slot) for slot in marker_frame.slots],
            "confidences": marker_frame.confidences,
            "detect_ms": round(detect_ms, 3),
        }

    pool = pool_rows(window)
    failed = sum(entry.failed for entry in window) >= FAILED_FRAMES_FAIL_RUN
    state = derive(pool, homography.centre_x).to_json()
    observation |= {"pool": pool, "failed": failed, "state": state}
    log.write(json.dumps(observation) + "\n")
    ws.send(json.dumps({"id": frame_id, "pool": pool, "failed": failed, "state": state}))


if __name__ == "__main__":
    main()
