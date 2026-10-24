# MuJoCo Plan — the loop's physics

The loop's frames must show the cart-pole moving the way the real rig
would, so the orchestrator needs real physics in place of the stub's
scripted sweep: MuJoCo, loaded with the exported MJCF, driven by setting
the cart actuator's `ctrl` and stepping, read back as `qpos`. This plan
adds a `physics` crate to the repo-root workspace that links the Rust
orchestrator directly to MuJoCo's C library (`mujoco.dll`), configured
by a `physics.toml` beside `Cargo.toml` (where MuJoCo is, which model to
load, its joint and actuator names). The orchestrator stub gains a
physics mode that steps MuJoCo between frames and sends the resulting
joint positions to the Unity player in the existing message format.

Status (end of 2026-10-22): **the physics leg works end to end on the
`<velocity>` model** — Done-when 1–5 pass (*Results — velocity model*
below); steps 1–7 are done. Nothing is committed. Decided by the user on
2026-10-22: in-process FFI, not a separate worker process; a `physics`
workspace crate; a repo-root `physics.toml` as its configuration; the
`toml` crate as the one new dependency; and the cart actuator is a
`<velocity>` servo (`ctrl` = target cart speed in m/s, sign = direction),
replacing the `<position>` servo the leg was first built and checked on
(*Results — position model*, kept as history).

Since (2026-10-24, committed; the *Superseded* notes below have the
detail): the crate's API is unchanged, its configuration is not.
`orchestrator.toml` (was `physics.toml`) holds only `mujoco_dir`, read
by `build.rs`; the MJCF and its three names come from the orchestrator's
`--model`, `--cart-joint`, `--pole-joint` and `--actuator`, which the
trainer fills from its `settings.toml` `[mjcf]`. The build copies
`mujoco.dll` beside the binary, so `target/release/orchestrator.exe`
runs without cargo. The MJCF states `<option timestep="0.002"
integrator="implicitfast"/>`, written by the Unity exporter from the
rig's StepJointSet (greyhound `da107c4`). `physics/README.md` documents
the crate.

**The planned loop is `synthetic-loop-dia.md`** (2026-10-23), the
source of truth: it supersedes the decisions marked *Superseded* below.
What this plan built and measured (*Results*) stays as the record of the
leg as it runs today.

## Done when

1. `cargo run -p orchestrator -- --physics --no-render --frames 1`
   builds, links `mujoco.lib` from `physics.toml`'s `mujoco_dir`, finds
   `mujoco.dll` at run time, and prints the MuJoCo version (`3.15.0`) and
   the model check line (step 4).
2. **Speed:** `--physics --no-render --control speed:0.2 --frames 50`
   (1 s): the pole's first excursion is negative; the cart's position at
   t = 0.98 s is within 1 mm of 0.2 × (0.98 − τ) m with τ ≈ 11.4 ms; and
   its speed (position difference between frames ÷ 0.02 s) settles at
   0.2 − 0.0005 m/s. **Clamp:** with `speed:0.6`, the speed settles at
   ≈ 0.48 m/s (the `ctrlrange`) until the slide's 0.45 m limit stops the
   cart. Step 6 says why these are the expected values.
3. **Free swing:** `--physics --no-render --start 0,1.0 --control zero --frames 250`:
   the pole's swing never exceeds 1.0 rad and its peaks shrink; the cart
   stays near its start. *Superseded (2026-10-23):* `--start` now takes
   the pole's angle alone (`--start 1.0`); the cart always starts at the
   rail's center (`synthetic-loop-dia.md`, 2).
4. **Deterministic:** the same command run twice writes byte-identical
   `states.csv` files.
5. **Rendered:** with the built player, `--physics --control sine:0.4,4 --frames 250`
   returns 250 frames, every one 728×544; the cart swings about
   ±0.4 × 4 / 2π ≈ ±0.255 m; spot-checked frames show the cart where
   `states.csv` says it is and the pole lagging the cart's motion.

## Results — velocity model (2026-10-22)

The model re-exported from Unity with `03-Cart.Slider.Actuator` as
`<velocity … ctrlrange="-0.48 0.48" forcerange="-98 98" … kv="100"
damping="0" armature="0.227">`, at the joint set's new STEP location
(`physics.toml` `model`). All runs from the repo root with
`cargo run -q -p orchestrator -- …`; physics-only output under
`target/physics-check-vel/`. Every run prints the model path it loaded.

- **Load — Done-when 1 passes.** Same model check line as before (two
  joints, ids 0 and 1; one actuator, id 0; timestep 0.002 s), from
  `…/Assets/Simulation/Robot/asy-cart-pole.step~/asy-cart-pole.xml`.
  MuJoCo 3.15.0's `compile.exe` also loads the file ("Done.").
- **Speed — Done-when 2 passes.** `speed:0.2`: the pole's first nonzero
  sample is −0.0071 rad at t = 0.02 s (negative). Cart at t = 0.98 s:
  0.19364 m, against 0.2 × (0.98 − 0.0114) = 0.19372 m predicted. The
  speed is 0.192 m/s by t = 0.04 s and holds at 0.1995 m/s from about
  0.9 s.
- **Clamp — Done-when 2 passes.** `speed:0.6`: the speed holds at
  0.4815–0.4819 m/s: the `ctrlrange` holds it near 0.48. It runs about
  0.002 m/s (0.4%) above 0.48 rather than the 0.0005 below that friction
  alone would give; not investigated — the pole's reaction force on the
  cart is the likely source. The cart reaches the slide's
  limit at t ≈ 0.96 s and stops at 0.4529 m. MuJoCo's limit is a soft
  constraint, so the cart passes the 0.45 m range by about 3 mm.
- **Free swing — Done-when 3 passes.** `--start 0,1.0 --control zero`,
  5 s: max |pole| = 1.0 (the start), and the peaks shrink by about
  0.08 rad each half-swing (0.911, 0.827, 0.744, 0.665, 0.587, 0.511,
  0.438, 0.366), a little faster than on the position model: the
  velocity servo damps the cart (`kv` acts as 100 N·s/m of damping
  around the commanded speed 0), and the pole's swing loses energy into
  it. The cart moves within ±2.8 mm (±0.16 mm on the stiff position
  servo) and ends 0.1 mm from its start: a speed command holds no
  position, but zero speed under `kv` keeps it from drifting.
- **Deterministic — Done-when 4 passes.** Two free-swing runs wrote
  byte-identical `states.csv` files.
- **Rendered — Done-when 5 passes.** `--player …/SimulationLoop.exe
  --physics --control sine:0.4,4 --frames 250`: 250 frames, every one
  `728x544`, player exit code 0. The cart swings from +0.258 m
  (t = 1.02 s) to −0.250 m (t = 3.02 s), against ±0.255 m predicted (the
  ~3 mm offset is the servo's lag during the first quarter-swing).
  Frames spot-checked against `states.csv`:
  - frame 45: cart +0.254 m, pole +0.20 rad. The cart is near the right
    end, slowing, and the tip swings right.
  - frame 150: cart −0.250 m, pole −0.05 rad. The cart is at the left
    end and the tip is slightly left.
- **Lint:** `cargo clippy --workspace --all-targets` clean.
- **Not measured:** speed of the loop.

## Results — position model (2026-10-22, superseded)

The leg as first built and checked, on the `<position>` actuator
(`ctrlrange` ±0.4 m, `forcerange` ±157 N, `kp` 8000, `dampratio` 1) at
the old model path `unity/projects/cart-pole/Assets/asy-cart-pole.step~/`.
The controls were then position targets (`hold:<m>`, `sine` as a
position sine); `hold:` no longer exists. All runs from the repo root
with `cargo run -q -p orchestrator -- …`; physics-only output under
`target/physics-check/`.

- **Load — Done-when 1 passes.** `--physics --no-render --frames 1`
  printed
  `MuJoCo 3.15.0  qpos 2 (02-Cart.Slider.joint 0, 04-Pole.Hinge.joint 1)  ctrl 1 (03-Cart.Slider.Actuator 0)  timestep 0.002 s`
  and `20 ms = 10 steps per frame`; exit code 0. This confirms R3 (`cargo
  run` finds `mujoco.dll` in the build script's `OUT_DIR` with nothing
  installed), the model check (P2), and R4 (the measured timestep is
  MuJoCo's 2 ms default). MuJoCo also logs its compile timing through
  the handler (`mujoco info: compile time (ms)`, ~100 ms wall clock); no
  `MUJOCO_LOG.TXT` was written.
- **Hold — Done-when 2 passes (after the check was corrected).** With
  `hold:0.2`, the pole's first nonzero sample is −0.132 rad at
  t = 0.02 s (negative, as predicted). The original 3 s check failed its
  cart bound: the last row read 0.2000260 m (2.6e-5 m off), because the
  pole was still swinging (±0.3 rad at 3 s) and its reaction force moves
  the cart by up to ~8e-5 m against `kp`. The 6.25 µm bound only holds
  once the pole is at rest. A 10 s run shows the pole stopping at about
  6 s; from 6.5 s on the cart is within 5e-10 m of 0.2 (2.1e-11 m at
  9.98 s). The pole then creeps very slowly (−0.0023 → −0.0020 rad over
  3.5 s), consistent with MuJoCo's frictionloss being a soft constraint
  rather than a hard stop.
- **Free swing — Done-when 3 passes.** `--start 0,1.0 --control zero`,
  5 s: max |pole| = 1.0 (the start), and successive peaks shrink by about
  0.07 rad each half-swing (0.923, 0.848, 0.775, 0.703, 0.633, 0.564,
  0.495, 0.428). An even drop per swing is what Coulomb friction
  (`frictionloss`) gives. Half-period ≈ 0.62 s. The cart stays within
  ±0.16 mm of 0.
- **Deterministic — Done-when 4 passes.** Two free-swing runs wrote
  byte-identical `states.csv` files (same SHA-256,
  `cb1f7fce…84dfdb7`).
- **Rendered — Done-when 5 passes.** `--player …/SimulationLoop.exe
  --physics --control sine:0.3,4 --frames 250`: the player connected,
  returned 250 frames, every one `728x544`, and exited with code 0. The
  cart follows the command closely (ctrl +0.300 → cart +0.300 at
  t = 1 s; ctrl 0 → cart +0.014 at t = 2 s, the lag of the position
  servo). Frames spot-checked against `states.csv`:
  - frame 12: cart +0.098 m, pole −0.20 rad. The cart is right of center
    and accelerating right, and the pole tip lags to the left.
  - frame 46: cart +0.296 m, pole +0.27 rad. The cart is near the right
    end and the tip swings right.
  - frame 150: cart −0.300 m, pole −0.11 rad. The cart is at the left
    end and the tip is slightly left.
- **Regression — step 7 passes.** With the names now from
  `physics.toml` (R1): the 60-state sweep (`all 60 frames 728x544`) and
  `--pose 0.2,0.5` (one `728x544` frame), both with player exit code 0.
  The guards work: `--control` without `--physics` is rejected by the
  argument parser, and `--frame-interval-ms 3` is rejected as "not a
  whole number of 0.002 s physics steps" (P4).
- **Not measured:** speed.

## Next

1. **Commit** (nothing is committed). New: `physics.toml`, `physics/`,
   `orchestrator/src/control.rs`, `orchestrator/src/sweep.rs`,
   `mujoco-plan.md`. Changed: root `Cargo.toml` and `Cargo.lock`,
   `orchestrator/Cargo.toml`, `orchestrator/src/args.rs`,
   `orchestrator/src/main.rs`. Removed: `orchestrator/src/pose.rs`
   (`Pose` moved to `physics`). These sit on top of the Unity leg's
   uncommitted files (`unity-player-plan.md`, *Next*).
2. ~~`loop-plan.md` wording~~ — done 2026-10-22 (*Consequence for
   `loop-plan.md`*, below).

## Facts this plan builds on

- **The model** `unity/projects/cart-pole/Assets/Simulation/Robot/asy-cart-pole.step~/asy-cart-pole.xml`
  (exported 2026-10-23 10:08; the exporter writes beside the STEP asset
  the joint set's `stepAssetPath` names, now
  `Assets/Simulation/Robot/asy-cart-pole.step`):
  - exactly two `<joint>` elements: slide `02-Cart.Slider.joint` (XML
    line 796; `axis="1 0 0"`, `range="-0.45 0.45"`, `frictionloss="0.05"`)
    and, nested deeper, hinge `04-Pole.Hinge.joint` (line 1176;
    `axis="0 -1 0"`, `limited="false"`, `frictionloss="0.02"`);
  - one actuator, `<velocity name="03-Cart.Slider.Actuator">` on the
    slide: `ctrlrange="-0.68 0.68"` with `ctrllimited="true"`,
    `forcerange="-98 98"`, `kv="100"`, `armature="0.227"`, `delay="0"`
    (each value's derivation: *The actuator's values*, below);
  - no `<option>` element, so MuJoCo's default timestep (2 ms) and
    integrator (Euler) apply;
  - `<compiler angle="radian" meshdir="meshes">`; the `meshes/` folder
    sits beside the XML;
  - moving mass 0.909 kg (cart subtree 0.507 kg + pole subtree
    0.402 kg, summed from the `<inertial mass>` values).
  A `<velocity>` actuator's `ctrl` is a **target cart speed in m/s**:
  force = `kv · (ctrl − q̇)` (MuJoCo 3.15.0 XML reference,
  `actuator/velocity`), clamped to `forcerange`. Nothing in the actuator
  holds a position: the slide's `range` is what stops the cart.
  The superseded `<position>` export (`ctrl` = target position in
  meters, `kp · (ctrl − q)`) is still at the old path
  `unity/projects/cart-pole/Assets/asy-cart-pole.step~/`.
- **MuJoCo 3.15.0 for Windows** is extracted at
  `D:\git\greyhound-snake\dist\mujoco\x86_64-windows\3.15.0` — the
  sibling greyhound checkout, the same one the Unity manifest's `file:`
  entries resolve to. `dist/` is gitignored there. Contents used here:
  `bin\mujoco.dll`, `lib\mujoco.lib` (import library),
  `include\mujoco\*.h`. Its imports are the four Visual C++ runtime DLLs
  plus Windows system DLLs (greyhound `mujoco-win.md`).
- **The C API reaches everything this leg needs through functions**
  (`include\mujoco\mujoco.h`, 3.15.0):
  - `mj_loadXML(filename, vfs, error, error_sz)` → `mjModel*` or NULL
    with the message in `error`;
  - `mj_makeData`, `mj_resetData`, `mj_forward`, `mj_step`,
    `mj_deleteData`, `mj_deleteModel`;
  - `mj_name2id(m, type, name)` → id, or −1 if not found;
  - `mj_stateSize(m, sig)`, `mj_getState(m, d, state, sig)`,
    `mj_setState(m, d, state, sig)` — copy chosen state components
    (`mjSTATE_TIME = 1`, `_QPOS = 2`, `_QVEL = 4`, `_CTRL = 64`;
    `mjtype.h`) between `mjData` and a caller's `double` buffer;
  - `mj_version()` (the header's `mjVERSION_HEADER` is `3015000`, "should
    match the library version as returned by mj_version()");
  - `mju_setLogHandler(handler)` with
    `typedef void (*mjfLogHandler)(const mjLogMessage*)`; `mjLogMessage`
    is `int level; int topic; char subject[1024]; const char* body;
    const char* func; const char* file; int line; mjtBool timestamp;`
    (`mjtype.h`; `mjtBool` is C `_Bool`, one byte), and `mjLOG_WARNING = 2`,
    `mjLOG_ERROR = 3`;
  - `mju_error` "does not return to caller"; `mju_warning` "returns to
    caller".
  So the reading and writing of `qpos`, `ctrl`, and `time` never depends
  on the layout of `mjModel` or `mjData` (huge structs with hundreds of
  fields).
- **Object type ids** (`mjtObj`, `mjtype.h`, counted from
  `mjOBJ_UNKNOWN = 0`): `mjOBJ_JOINT = 3`, `mjOBJ_ACTUATOR = 19`.
- **`mjtNum` is `double`** unless `mjUSESINGLE` is defined (`mjtype.h`);
  the release DLL is the default (double) build.
- **The sign convention, verified in the Unity leg** (`unity-player-plan.md`
  *Results*): `+x` cart is right in the frame, and a small positive pole
  angle puts the tip toward `+x` — the player renders MuJoCo's `qpos`
  the way MuJoCo `simulate` shows it.
- **The orchestrator stub** (`orchestrator/`): `Pose { cart, pole }` →
  `StateMessage { id, joints: [{name, qpos}] }` → one JPEG back per
  state, lockstep, checked against 728×544, saved as
  `frame_NNNNNN.jpg` in `--out`. Modes today: the 60-state sweep and
  `--pose`. The joint names on the wire come from `--cart-joint` /
  `--pole-joint` (defaults: the MJCF names). It runs under
  `#[tokio::main]`; the workspace denies `unwrap`, `expect`, and `panic`
  (clippy) and `unused_must_use`; edition 2024. Workspace dependencies
  today: `anyhow`, `clap`, `futures-util`, `serde`, `serde_json`,
  `tokio`, `tokio-tungstenite` — no TOML parser.
- **Toolchain:** `stable-x86_64-pc-windows-msvc`, cargo/rustc 1.99.0. No
  `clang` on `PATH` and no `LIBCLANG_PATH`, so `bindgen` is not usable
  without installing something.

## The actuator's values

`03-Cart.Slider.Actuator` stands in for the stepper, the DRV8825 driver
and the belt: the firmware turns the command into a step-pulse rate,
which the motor follows, so the model is a speed servo on the slide. Its
values live in the Unity asset
`Assets/Simulation/MuJoCo/03-Cart.Slider.Actuator.asset` and reach the
MJCF only through a re-export. Each one below follows from the inputs by
hand.

**Inputs:**

| Input | Value | Source |
|---|---|---|
| Motor | 23HS32-4004S: 1.8° (200 full steps/rev), 4.0 A rated, 2.4 N·m holding torque, 0.9 Ω and 3.4 mH per phase | third-party listings (2026-10-23); StepperOnline's own page refused the fetch, so unconfirmed against the maker |
| Phase current | 2.5 A, assumed | the DRV8825's maximum output current; the real setting is to be measured |
| Supply | 30 V DC, the bench supply (24 V first planned) | user, 2026-10-24 |
| Pulley | 48T GT2, 2 mm pitch: 96 mm per turn, pitch radius r = 96 / 2π = 15.28 mm | the CAD part `pully-gt2-48t-6w-8b` |
| Moving mass | 0.909 kg (cart 0.507 + pole 0.402) | the MJCF's `<inertial mass>` values (*Facts*) |

**Values:**

- **`ctrlrange` ±0.85 m/s** — the motor's estimated corner speed, the
  speed where the winding inductance stops the current from reaching
  its setting within one step, so torque falls off above it. Full-step
  rate ≈ V ÷ (2·L·I) = 30 ÷ (2 × 0.0034 × 2.5) ≈ 1,765 steps/s = 8.82
  rev/s; × 96 mm = 0.847 m/s, set as 0.85 in the StepJointSet and the
  exported MJCF (user, 2026-10-24). An estimate, not a measurement; at
  the 24 V first planned it was 0.68 m/s (2026-10-23).
  It replaces 0.48 m/s, the motor's only rated speed (300 RPM = 5 rev/s
  × 96 mm).
- **`forcerange` ±98 N** — the torque at the set current, at the belt.
  Holding torque scaled by current: 2.4 × 2.5 / 4.0 = 1.5 N·m; ÷ r =
  1.5 / 0.01528 = 98 N. This is low-speed torque: a stepper's torque
  falls off above the corner speed, which a constant `forcerange`
  cannot follow, so near ±0.85 m/s the model is stronger than the
  motor.
- **`armature` 0.227 kg** — the rotor's inertia felt at the cart:
  J ÷ r². Back-computed, J = 0.227 × 0.01528² = 5.3 × 10⁻⁵ kg·m²
  (530 g·cm²); the datasheet figure it came from is not recorded here.
  It adds to the moving mass: m_eff = 0.909 + 0.227 = 1.136 kg.
- **`kv` 100 N·s/m** — how hard the servo pushes per m/s of speed
  error: force = kv · (ctrl − q̇). It is a modelling choice, not a motor
  constant, and why 100 was picked is not recorded. What it gives,
  checkable by hand: the speed follows a command with time constant
  τ = m_eff / kv = 1.136 / 100 ≈ 11 ms; a steady speed falls short by
  frictionloss / kv = 0.05 / 100 = 0.0005 m/s; the force reaches the
  98 N clamp only at a speed error above 98 / 100 = 0.98 m/s. A real
  stepper below its corner speed follows its pulse rate with no speed
  error at all, so a large kv is the closer model.
- **`gear` 1** — the actuator drives the slide directly, so `ctrl` is
  in m/s and the force in N at the cart; the pulley's ratio is already
  folded into `forcerange` and `armature`.
- **`ctrllimited`, `forcelimited` true** — MuJoCo clamps the command to
  `ctrlrange` and the force to `forcerange`, so a command beyond the
  motor's range cannot reach the physics.
- **`damping` 0** — the slide's friction is the joint's `frictionloss`
  (0.05 N), not the actuator's.
- **`delay` 0** — the command delay D belongs to the orchestrator's
  scheduling (`orchestrator-plan.md`), not to the actuator.
- **`kp`, `dampratio`, `timeconst`, `cranklength`, `lengthrange`,
  `inheritrange`** — stored in the asset but not used by a
  `<velocity>` actuator.

## Settled decisions

- **In-process FFI** (user, 2026-10-22): the orchestrator process links
  `mujoco.dll` and calls it directly; no physics-worker process, no wire
  format for physics. A process boundary can be added later around the
  same `Physics` type.
- **A `physics` crate in the workspace** (user, 2026-10-22): a library
  crate, member of the repo-root `Cargo.toml`. All `unsafe` and the build
  script live there; the orchestrator stays safe code and only sees
  `Physics` and `PhysicsConfig`.
- **`physics.toml` at the repo root, beside `Cargo.toml`** (user,
  2026-10-22): the physics crate's configuration — where MuJoCo is, the
  model to load, its joint and actuator names (*`physics.toml`* below).
  *Superseded (user, 2026-10-23):* renamed **`orchestrator.toml`**, the
  more intuitive name, since the orchestrator is the program it
  configures. It now holds only `mujoco_dir`, a build input: `model`
  and `[names]` moved to the trainer's `settings.toml` `[mjcf]`, and
  reach the orchestrator as `--model`, `--cart-joint`, `--pole-joint`
  and `--actuator` (user, 2026-10-23). This supersedes R1 and the run-time
  reader in the rules below.
- **Hand-written declarations, no bindgen.** About a dozen `extern "C"`
  functions, each checked by hand against the 3.15.0 header.
- **Function-only access.** `mjModel` and `mjData` are opaque pointers;
  no Rust mirror of either struct. `qpos`, `qvel`, `ctrl`, and `time` go
  through `mj_getState` / `mj_setState`. The only mirrored struct is the
  eight-field `mjLogMessage`.
- **The orchestrator stub grows a physics mode**; the sweep and `--pose`
  stay as the renderer's smoke tests. The message to the player is
  unchanged. *Superseded (2026-10-23):* physics is the only mode, so
  `--physics` is gone; the sweep and `--pose` went with the player's
  socket to the orchestrator (Unity now connects to the synthetic Orin,
  `synthetic-loop-dia.md` 6–7). `--no-render` stays: physics and
  `states.csv` only.

## `physics.toml`

*Now `orchestrator.toml` (user, 2026-10-23, Settled decisions); the
content and rules below are unchanged.*

```toml
# Read by the physics crate at build time (mujoco_dir: what to link) and
# at run time (everything). Relative paths resolve against this file's
# folder, the repo root.

# A MuJoCo 3.15.0 release tree: bin\mujoco.dll, lib\mujoco.lib.
# Here: the sibling greyhound checkout (greyhound mujoco-win.md).
mujoco_dir = "../greyhound-snake/dist/mujoco/x86_64-windows/3.15.0"

# The MJCF the Unity exporter writes beside the STEP asset the joint set
# names (stepAssetPath: Assets/Simulation/Robot/asy-cart-pole.step).
model = "unity/projects/cart-pole/Assets/Simulation/Robot/asy-cart-pole.step~/asy-cart-pole.xml"

# MJCF names, as in the model's <joint name> / actuator name. The
# joint names are also the wire keys of the player's state message.
[names]
cart_joint = "02-Cart.Slider.joint"
pole_joint = "04-Pole.Hinge.joint"
actuator   = "03-Cart.Slider.Actuator"
```

Rules:

- **One file, two readers.** `physics/build.rs` reads
  `<workspace root>/physics.toml` (`CARGO_MANIFEST_DIR/..`) for
  `mujoco_dir`, and emits its absolute path to the crate as
  `cargo:rustc-env=PHYSICS_TOML=<path>`; `PhysicsConfig::load()` reads
  that same path at run time. So the orchestrator never looks for the
  file relative to the current directory, and the MuJoCo it links is
  always the one the file names. `cargo:rerun-if-changed=<path>` makes
  an edit to `mujoco_dir` relink on the next `cargo run`.
- **`mujoco.dll` beside the binaries** (2026-10-23): the trainer
  launches `target/release/orchestrator.exe` without cargo, so the build
  script also copies the DLL into `target/<profile>/`, where Windows
  looks first. `mujoco_dir` is therefore a build input only: the run
  uses the MuJoCo the binary was built with.
- **Paths resolve against the file's folder**, not the current
  directory (Cargo's own rule for paths in its config files), so the
  file is committable as written: both paths are relative.
- **Not in the file:** the MuJoCo version. The hand-written
  declarations are only valid for the 3.15.0 header, so the expected
  version is a constant in the code beside them (P1), not configuration.
  Per-run values (control, start pose, frame interval) stay command-line
  options.

## Requirements and recommendations

Requirements (the leg is wrong without them):

- **P1 — version guard.** At load, `mj_version()` must equal `3015000`,
  or the run stops. The hand-written declarations are only valid for
  the header they were checked against.
- **P2 — model check.** At load, both joint names and the actuator name
  must resolve (`mj_name2id` ≠ −1), `mj_stateSize(QPOS)` must be 2, the
  two joint ids must be 0 and 1, `mj_stateSize(CTRL)` must be 1, and the
  actuator id must be 0. Rule that makes this hand-checkable: MuJoCo
  hands out `qpos` slots in joint order, and a slide or hinge takes
  exactly one, so with two such joints and `nq = 2`, **a joint's `qpos`
  index is its joint id**. The same holds for actuators and `ctrl`. Any
  other model (a third joint, a free joint, a second actuator) fails the
  check instead of reading the wrong slot.
- **P3 — warnings fail the run.** MuJoCo reports instability (and its
  automatic reset of the simulation) as warnings. The log handler counts
  warnings; a run that saw any exits with code 1 after its last frame,
  because a silent reset would make the truth log and the frames lie.
- **P4 — MuJoCo's clock only.** Simulated time advances only through
  `mj_step`; the frame interval is a whole number of physics steps
  (`synthetic-loop-dia.md`). An interval that is not a whole multiple
  of the timestep is rejected, not rounded. *Superseded
  (`synthetic-loop-dia.md`):* there is no frame interval. Time still
  advances only through `mj_step`, a fixed 2 ms each; the arrival step
  is the first step whose time is later than capture time + the Orin's
  RND(MIN, MAX) ms latency (8–15 ms today), so the latency is rounded up to the next step
  by design.
- **P5 — no panic across FFI.** The log handler is called from C; it
  must not panic (write with `writeln!` to `stderr` and ignore the
  result, not `eprintln!`).

Recommendations (choices, with the reason):

- **R1 — `physics.toml` names the joints for every mode.** The stub's
  `--cart-joint` / `--pole-joint` duplicate `[names]`; with both, the
  physics mode could read `qpos` by one name and send it under another.
  The flags are dropped, and the sweep and `--pose` modes take the wire
  keys from `[names]` too (reading the file, not loading MuJoCo).
- **R2 — the `toml` crate.** Parsing `physics.toml` needs a TOML parser,
  in both `build.rs` and the library: `toml` (with `serde`, already a
  workspace dependency) as a workspace dependency, used by `physics` as
  both a dependency and a build-dependency. It is the one new crate this
  leg adds: `toml = "1.1.8"`, approved by the user 2026-10-22. The build
  script needs only `mujoco_dir`, so it reads the file as a plain
  `toml::Table`; the full, strict parse is `PhysicsConfig`'s.
- **R3 — the DLL is copied, not installed.** The build script copies
  `mujoco.dll` and `mujoco.lib` into its `OUT_DIR` and adds `OUT_DIR` to
  the link search path. Cargo puts link-search paths inside `target/` on
  `PATH` for `cargo run` and `cargo test` (Cargo book, *Dynamic library
  paths*), so the DLL is found with nothing installed and nothing
  written outside `target/`. Expected; step 1 confirms it. Running
  `target\debug\orchestrator.exe` directly, outside cargo, would not
  find it — not needed in this leg.
- **R4 — the timestep is measured, not read.** `mjModel.opt.timestep`
  is a struct field. Instead, `Physics::load` resets, takes one
  `mj_step`, reads `time` (which then equals the timestep), and resets
  again. One extra step at load time keeps the function-only decision
  (*Settled decisions*) intact.

## Steps

1. **Workspace and build script.**
   - `physics.toml` at the repo root, as above.
   - `toml` added to `[workspace.dependencies]` (R2, after the user's OK).
   - New workspace member `physics/` (library; `version`, `edition`, and
     lints from the workspace), added to `members` in the root
     `Cargo.toml`; `serde` and `toml` as dependencies and
     build-dependencies.
   - `physics/build.rs`: read `physics.toml` (*`physics.toml`*, rules);
     resolve `mujoco_dir`; fail with a message naming the resolved path
     if `bin\mujoco.dll` or `lib\mujoco.lib` is missing; copy both into
     `OUT_DIR`; emit `cargo:rustc-link-search=native=<OUT_DIR>`,
     `cargo:rustc-link-lib=dylib=mujoco`,
     `cargo:rustc-env=PHYSICS_TOML=<absolute path>`, and
     `cargo:rerun-if-changed` for `physics.toml` and the two MuJoCo
     files.
   *Check:* `cargo build -p orchestrator` links; the Done-when 1 command
   prints the version (proves the DLL is found at run time).
2. **`PhysicsConfig`** (`physics/src/config.rs`): the file's fields as a
   `serde::Deserialize` struct: `mujoco_dir`,
   `model`, `names { cart_joint, pole_joint, actuator }`.
   `PhysicsConfig::load()` reads `env!("PHYSICS_TOML")` and resolves
   both paths against that file's folder. An unknown key is an error
   (`deny_unknown_fields`), so a misspelt key cannot silently fall back.
3. **FFI declarations** (`physics/src/ffi.rs`, private to the crate).
   - Opaque `mjModel`, `mjData`, `mjVFS` (zero-sized `#[repr(C)]`
     structs, used only behind pointers).
   - `#[repr(C)] mjLogMessage`, all eight fields as in *Facts*.
   - `unsafe extern "C"` block (edition 2024 syntax): the functions in
     *Facts*, signatures copied from the 3.15.0 header (`int` → `i32`,
     `mjtNum` → `f64`, `const char*` → `*const c_char`).
   - Constants `mjOBJ_JOINT = 3`, `mjOBJ_ACTUATOR = 19`,
     `mjSTATE_TIME = 1`, `mjSTATE_QPOS = 2`, `mjSTATE_CTRL = 64`,
     `mjLOG_WARNING = 2`, `mjLOG_ERROR = 3`,
     `MJ_VERSION = 3015000` — each with its header line as the comment.
4. **`Physics`** (`physics/src/lib.rs`, safe API).
   - `Physics::load(config: &PhysicsConfig)`: install the log handler
     (P3, P5); P1; `mj_loadXML` on `config.model` with a 1000-byte error
     buffer (a NULL model becomes an error carrying that text);
     `mj_makeData`; P2 with `config.names`; measure the timestep (R4).
     The model path must be UTF-8 (it becomes a C string). Prints one
     line, e.g.
     `MuJoCo 3.15.0  qpos 2 (02-Cart.Slider.joint 0, 04-Pole.Hinge.joint 1)  ctrl 1 (03-Cart.Slider.Actuator 0)  timestep 0.002 s`.
   - `reset(start: Pose)`: `mj_resetData` (which zeroes `qvel` and
     `ctrl`), set `qpos` = (cart, pole) through `mj_setState`,
     `mj_forward` so derived quantities match the new state.
   - `set_ctrl(ctrl)`: rejects a non-finite value; writes `ctrl` in the
     actuator's units — for the cart's `<velocity>` actuator, target cart
     speed in m/s. Out-of-range values are clamped by MuJoCo itself
     (`ctrllimited="true"`, ±0.68 m/s); keeping the command inside the
     motor's speed range is the firmware's job in a later leg
     (`synthetic-arduino-plan.md`).
   - `step(n)`: `n` × `mj_step`.
   - `time()`, `pose() -> Pose`: one `mj_getState` each.
   - `warnings()`: the handler's count.
   - `Drop`: `mj_deleteData`, then `mj_deleteModel`.
   - `Physics` holds raw pointers, so it is neither `Send` nor `Sync`;
     it lives in the main task (`#[tokio::main]` runs it with
     `block_on`, which needs no `Send`).
   - The handler prints `mujoco warning: <subject>` /
     `mujoco error: <subject>`. An error ends the process inside MuJoCo
     (`mju_error` does not return); the player then sees its socket
     close and quits on its own, as it already does.
   - `Pose` moves from `orchestrator` into `physics` (the orchestrator
     re-uses it), so the crate needs no type of its own for a state.
5. **Orchestrator physics mode.**
   - `physics` added to the orchestrator's dependencies.
   - New options: `--physics` (turns physics mode on: `PhysicsConfig::load`,
     `Physics::load`); `--control zero | speed:<m_s> | sine:<amp_m_s>,<period_s>`
     (default `zero`; target cart speeds); `--start <cart>,<pole>`
     (default `0,0`);
     `--frame-interval-ms` (default 20, placeholder for T);
     `--no-render` (no player, no socket: physics and `states.csv` only;
     `--player` becomes `required_unless_present_any = [no_launch, no_render]`).
     `--cart-joint` / `--pole-joint` are removed; every mode takes the
     names from `PhysicsConfig` (R1).
   - `Control` (`orchestrator/src/control.rs`): `at(t) -> f64` in m/s —
     zero; the constant; `amp · cos(2π t / period)`. The cosine makes the
     cart's position (the speed's integral) a sine centered on the start,
     amplitude `amp · period / 2π`; a sine speed would integrate to
     `1 − cos` and push the cart to one side.
   - **Tick loop** (frame `i` is captured at `t = i · T`):
     read `pose()` → send it to the player and check the frame (existing
     code) → append a `states.csv` row → `set_ctrl(control.at(t))` →
     `step(T / timestep)`. The command is held for the whole interval
     and takes effect at the capture: D = 0. The real D scheduling
     belongs to the orchestrator plan. *Superseded
     (`synthetic-loop-dia.md`):* MuJoCo steps every 2 ms on the old
     command from the capture to the arrival step, frozen there until
     the Orin's reply is in; the new `ctrl` is set at the arrival step,
     which is also the next capture.
   - `states.csv` in `--out`: header `id,time,ctrl,cart,pole` (since
     the firmware leg, `id,time,ctrl,cart,pole,command`: `command` is
     the pulse-rate packet the scripted `ctrl` speed becomes,
     `synthetic-arduino-plan.md`), values
     written with Rust's `{}` for `f64` (shortest text that reads back
     to the same number), so files compare byte for byte (Done-when 4).
     *Superseded (`synthetic-loop-dia.md`, Truth log):* one row per
     observation — id, capture time, the true cart and pole at capture,
     and the 4 observed `(x, y)` of the pool's newest entry.
   - After the last frame: exit code 1 if any frame broke the 728×544
     contract (existing) or `warnings() > 0` (P3).
6. **Physics-only checks** (no Unity): the Done-when 1–4 commands.
   Why the expected values are expected:
   - *Speed, position:* the servo pulls the speed toward the command with
     time constant τ = m_eff / kv, where m_eff = 0.909 kg moving mass +
     0.227 kg reflected rotor (`armature`) = 1.136 kg, so τ = 1.136 / 100
     ≈ 11.4 ms. A first-order lag behind a constant speed v leaves the
     position v·τ short of v·t, so at t = 0.98 s the cart is at
     0.2 × (0.98 − 0.0114) ≈ 0.1937 m. The force needed (≈ 0.2 / 0.0114
     × 1.136 ≈ 20 N at the start) stays inside ±98 N, so the clamp does
     not slow it.
   - *Speed, settled value:* at a steady speed the servo's force
     `kv · (ctrl − q̇)` balances only the slide's `frictionloss` (0.05 N),
     so the speed falls short by 0.05 / 100 = 0.0005 m/s.
   - *Speed, pole:* the cart accelerates toward `+x`; a hanging pole
     lags behind, so its tip first swings toward `−x`, which is a
     negative angle (*Facts*, sign convention).
   - *Clamp:* `ctrlrange` ±0.48 m/s caps the command, so 0.6 m/s
     arrives as 0.48; the cart keeps going until the slide's 0.45 m
     `range` stops it, since a speed servo holds no position.
   - *Free swing:* with the command at 0 nothing adds energy; the
     hinge's `frictionloss` (0.02) and the servo's damping of the cart
     (`kv` around speed 0) take some each swing, so the peaks can only
     shrink. The cart moves only as much as the pole's reaction force
     pushes it against `kv`.
   *Fail:* a cart position off the τ prediction, a positive first
   excursion, a clamp other than 0.48, or growing peaks means a units,
   index, or sign bug in `physics` — fixed before anything is rendered.
7. **Rendered run** (Done-when 5): `--player unity/projects/cart-pole/Build/SimulationLoop.exe`
   with `--physics --control sine:0.4,4 --frames 250` (5 s simulated,
   1¼ periods). The cart's position swings ±0.4 × 4 / 2π ≈ ±0.255 m,
   inside the ±0.45 m slide; peak speed 0.4 m/s is inside the 0.48 m/s
   `ctrlrange`; peak acceleration 0.4 × 2π/4 ≈ 0.63 m/s² needs ≈ 0.7 N
   at 1.136 kg, well inside ±98 N. *Check:* 250 frames, all 728×544;
   frames spot-checked against their `states.csv` rows. The sweep and
   `--pose 0.2,0.5` are re-run once to confirm they still pass with the
   names from `physics.toml` (R1).

## Not in this plan

The T and D scheduling beyond a fixed interval and D = 0, the virtual
Arduino (microsteps → `ctrl`), the detector, the policy and the 6-frame
window, episodes (start-state randomization, end conditions, reset
between episodes), the reward, the training side and how it talks to the
orchestrator, a physics-worker process, pooling, running the
orchestrator executable outside `cargo`, and any change to the MJCF or
its exporter.

## Placeholders

- **T = 20 ms** (`--frame-interval-ms` default) — until the Pi's frame
  interval is measured. *Superseded
  (`synthetic-loop-dia.md`):* there is no frame interval; each capture
  goes out at the previous observation's arrival step.
- **The scripted controls** (`speed:0.2`, `speed:0.6`, `sine:0.4,4`,
  start `0,1.0`) are test values for bring-up stages 1–2
  (`trainer-plan.md`, *Bring-up*), not training settings.
- **The actuator's values** (±0.85 m/s, ±98 N, `kv` 100, `armature`
  0.227 kg; derivations in *The actuator's values*) rest on an assumed
  2.5 A phase current and an estimated corner speed; the real top speed
  and the DRV8825's actual current setting are to be measured.

## Consequence for `loop-plan.md` (retired 2026-10-23)

Its *Targets* item 2 ("Physics worker") and part 1's "No physics worker
process exists" describe the worker-process shape. Under this leg's
decision, physics is a library inside the orchestrator. Updated
2026-10-22: part 1 now describes the `physics` crate and the stub's
`--physics` mode, and *Targets* item 2 reads "Physics", in-process.

## References

- `synthetic-loop-dia.md` — the planned loop, the source of truth for
  the decisions marked *Superseded*
- `unity-player-plan.md` — the renderer, the message format, the sign
  convention check
- MuJoCo 3.15.0 headers: `include\mujoco\mujoco.h`, `mjtype.h`
- greyhound `mujoco-win.md` — the Windows install
- Cargo book: *Build scripts* (`rustc-link-search`, `rustc-link-lib`,
  `rustc-env`, `rerun-if-changed`), *Environment variables* (dynamic
  library paths)
