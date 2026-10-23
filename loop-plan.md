# Loop Plan — the simulated robot loop (coarse master plan)

The balance policy has to learn from what the real robot will see —
camera frames, not true joint state — so it trains in a loop that runs
the robot's own pipeline against a simulated rig: MuJoCo moves the
cart-pole, Unity renders the camera frame, the Pi's code detects the
markers and decides, the Arduino's code turns the decision into motor
steps, and those steps move the MuJoCo cart. This document fixes the
parts, their inputs and outputs, and how one turn of the loop runs. Each
part to be built gets its own implementation plan; this plan only says
what each part must do.

Status: **plan, nothing in the loop runs yet** (2026-10-22). "Exists"
below names what is landed today; everything else is to be built.

## Principles

- **Simple first.** One loop, one instance of each part, proven working
  end to end before anything is scaled. Scaling (below) is anticipated,
  not designed.
- **Live and virtual are one source.** The Pi's code and the Arduino's
  firmware are each one Rust source built two ways: `live` for the
  hardware, `virtual` for the loop. Only the edges (where frames come
  from, where commands go) differ. **Exception: object detection on the
  robot runs in Python** (user, 2026-10-23) — on the Orin, Python with
  OpenCV pre-processing and TensorRT ran 11.9–14.5 ms per frame against
  18.7–19.1 ms for the Rust POC in the same session
  (`detector-plan.md`, *Results*).
- **The robot's computer is a Jetson Orin Nano Super** (user,
  2026-10-23), replacing the Raspberry Pi 5: its GPU runs the detector
  in 8–10 ms per frame with TensorRT FP16 against 80–125 ms for the best
  Pi 5 options (`detector-plan.md`, *Results*). "The Pi" in this plan
  and the leg plans names that computer's role (the robot side of the
  loop), not the board. Its code is built on the device: the repo is
  cloned there and compiled (user, 2026-10-22, for the Pi; unchanged
  for the Orin). Requirement: every dependency of the robot-side code
  builds there (aarch64 Linux, JetPack 6.2.3: CUDA 12.6, TensorRT
  10.3).
- **The robot sees only pixels.** The Pi side receives rendered frames
  and nothing else. The true joint state (`qpos`) goes only to the
  orchestrator, for logs and the reward.
- **MuJoCo's clock is the authority.** Simulated time advances only by
  physics steps; every delay in the loop is expressed in simulated time.

## The parts

| # | Part | In | Out | Real-robot counterpart |
|---|---|---|---|---|
| 0 | **Orchestrator** | — | the tick, episode start/end, logs, process supervision | none (sim only) |
| 1 | **Physics** — MuJoCo + the exported MJCF | actuator control value `ctrl` | true joint positions `qpos` | the physical rig |
| 2 | **Renderer** — Unity player + `StateServer` | joint positions | 728×544 greyscale JPEG, q90 | Pi GS camera, frame reduced to 728×544 greyscale |
| 3 | **Detector** — virtual Pi | JPEG | 4-slot marker frame | same code on the Pi |
| 4 | **Policy** — virtual Pi | the last 6 marker frames, each with its age | one signed float | same code on the Pi |
| 5 | **Virtual Arduino** — firmware, `virtual` build | signed float | `ctrl` in meters | the Arduino + stepper + belt |

What exists today, per part:

- **0 Orchestrator** — nothing.
- **1 Physics** — the MJCF at
  `unity/projects/cart-pole/Assets/asy-cart-pole.step~/asy-cart-pole.xml`
  (slide `02-Cart.Slider.joint` ±0.45 m; hinge `04-Pole.Hinge.joint`,
  unlimited; `<position>` actuator `03-Cart.Slider.Actuator`, `ctrlrange`
  ±0.4 m, `forcerange` ±157 N). It loads in MuJoCo 3.15.0 `simulate` on
  this PC with the joints and actuator working (validated by the user
  2026-10-22; install per greyhound `mujoco-win.md`). At hinge angle 0
  the pole hangs down, so **upright is θ = π**. The `physics` crate
  (repo-root workspace, configured by `physics.toml`) runs it in-process
  through `mujoco.dll`: it loads the model, sets the actuator's `ctrl`,
  steps, and reads `qpos`. The orchestrator stub's `--physics` mode
  drives the Unity player with it, writing the true states to
  `states.csv` (`physics-plan.md`, *Results*, 2026-10-22; uncommitted).
  There is no separate physics-worker process, by decision.
- **2 Renderer** — `StateServer.cs` runs in the Editor: TCP on
  `127.0.0.1:5555`, one JSON object per line in (joint names + `qpos`),
  4-byte little-endian length + JPEG out. Today it renders **RGB at
  960×540** (code defaults), q90; the contract here is 728×544
  greyscale. No standalone player build exists.
- **3 Detector** — a trained model:
  `training/runs/cp2-640-base/weights/best.onnx` (YOLO11n, trained on
  `datasets/generated`, `channels: 1` greyscale, `imgsz` 640;
  2026-10-17). No Rust host exists.
- **4 Policy** — nothing; this is what training produces.
- **5 Virtual Arduino** — nothing yet; a one-source `live`/`virtual`
  Rust firmware was confirmed feasible by the user.

## Contracts between the parts

- **Frame (2 → 3):** 728×544 greyscale JPEG, quality 90 — exactly half
  the IMX296's native 1456×1088 (`DATASETS.md`, *Image spec*).
- **Marker frame (3 → 4):** 4 slots in class order — 0 Left (tag 376),
  1 Right (596), 2 Pivot (920), 3 End (937), the repo's class-id
  contract. Each slot is the box center `(x, y)`, normalized to [0, 1],
  or `(-1, -1)` when that class was not detected. `-1` lies outside the
  valid range, so "not detected" can never be a real position. When a
  class has several boxes, the slot takes the highest-confidence one.
  The flag means *not detected*, whatever the cause (occluded, missed,
  out of frame).
- **Policy input (→ 4):** the last **N = 6** marker frames, each
  prefixed by its **age** — seconds before the newest frame (newest = 0,
  older negative). 6 × (1 + 8) = **54 floats**. Ages are relative so the
  input never depends on absolute clock values, and the policy sees the
  real spacing when frames are uneven.
- **Command (4 → 5):** one signed float; sign = direction, magnitude =
  amount.
- **Actuator value (5 → 1):** the firmware turns commands into whole
  microsteps (fractional remainder carried, clamped to the travel
  envelope). The `virtual` build's output layer — and only it — converts
  the running microstep total to `ctrl` in meters: total × (meters per
  microstep). Meters per microstep = belt travel per pulley revolution
  ÷ (motor full steps per revolution × driver microstep setting). The
  belt travel is 96 mm: the CAD pulley `pully-gt2-48t-6w-8b` has 48
  teeth at GT2's 2 mm pitch. The motor's steps per revolution and the
  driver's microstep setting are to be confirmed (see *Placeholders*).
  The `live` build emits step pulses instead. `ctrl` stays within the
  MJCF's ±0.4 m `ctrlrange`.
- **Truth (1 → 0):** `qpos` to the orchestrator only — logs and reward.

## Timing

- **MuJoCo's `data.time` is the clock.** It advances 2 ms per physics
  step (the MJCF sets no `<option>`, so MuJoCo's default timestep
  applies) and by nothing else.
- **T — frame interval, in simulated time.** A frame is taken every
  `T / 2 ms` physics steps. T must equal the real Pi's frame interval;
  until that is measured it is a placeholder.
- **D — command delay, in simulated time.** The span from frame capture
  to the command reaching the Arduino: detection + policy + WiFi. A new
  command takes effect `D / 2 ms` physics steps after its frame's
  capture — possibly mid-tick. Until the old command is replaced,
  physics keeps running on it: the cart keeps tracking the old target
  and the pole keeps its momentum. Placeholder **D = one tick** until
  measured on the Pi; training later randomizes D in a band around the
  measured value, because the policy cannot observe D directly.
- **Wall clock: as fast as possible.** The loop runs as fast as the
  machine allows; the 3090 finishing detection sooner than the Pi
  changes wall-clock speed only, never simulated time.

## One turn of the loop (tick n)

1. **Physics** steps through simulated time to frame n's capture,
   applying the pending command when its D elapses → `qpos` at capture.
2. **Renderer** sets cart and pole from `qpos` → 728×544 greyscale JPEG.
3. **Detector** → 4-slot marker frame.
4. **Policy** takes the last 6 marker frames with their ages → signed
   float.
5. **Virtual Arduino** → microsteps → `ctrl`, which becomes the pending
   command, effective D after frame n's capture.
6. **Orchestrator** logs `qpos` (and the reward when training), checks
   for episode end, then starts tick n+1 or resets.

## Transport

- **Sim:** every link goes through the orchestrator.
- **Real:** the Pi sends command packets to the Arduino over WiFi.
- Per-link wire formats and process boundaries belong to the
  per-target implementation plans. The only defined format today is
  `StateServer`'s (above).

## Targets — each gets its own implementation plan

1. **Orchestrator** — tick and clock scheduling (T, D), episode
   control, logs, process supervision.
2. **Physics** — MuJoCo stepping on the exported MJCF, in-process: the
   `physics` crate linked into the orchestrator, not a worker process;
   accepts `ctrl`, reports `qpos`. Plan: `physics-plan.md`.
3. **Unity player** — a standalone build of the renderer at 728×544
   greyscale, q90.
4. **Pi code, `live` + `virtual`** — JPEG decode, detector inference
   and slot filling, the 6-frame window, the policy slot. Detection on
   the robot is Python (*Principles*, exception). The object
   detection crate's parameters are configured outside source code, in
   `ObjectDetection.toml` at the repo root (user, 2026-10-22), as
   `physics.toml` configures the physics crate.
5. **Firmware, `live` + `virtual`** — command accumulation, clamp,
   and the `virtual` output layer producing `ctrl`.

## Bring-up — what fills the policy slot

Each stage must pass before the next, so a failure has one place to look.

1. **Zero command** — the cart holds still, the pole hangs or swings.
   Proves the plumbing: every part runs, frames return, the Pivot/End
   slots track the swinging pole, episodes reset.
2. **Scripted command** — a slow back-and-forth. Proves the command
   path (float → firmware → `ctrl` → physics → render) and lets the
   detector's slots be compared with the true positions the
   orchestrator holds.
3. **Trained policy** — only after 1 and 2 pass; a failure here is
   learning, not plumbing.

## Training settings (tuned during training, not loop structure)

Episode start state (pole near θ = π, cart near center), end conditions,
and the reward.

## Placeholders and checks to close

- **T** — measure the Pi's camera + detector frame interval.
- **D** — measure detection + policy + WiFi on the Pi.
- **Pi image reduction** — the sensor is colour; the Pi's reduction to
  728×544 greyscale must use the same method as the renderer's, or the
  real frames will not look like the training frames.
- **GPU vs CPU inference** — the same ONNX model on the 3090 and on the
  Pi can differ slightly; compare once.
- **Meters per microstep** — the NEMA-23's full steps per revolution and
  the driver's microstep setting, for the firmware's `virtual` output.

## Later — scaling (only after the simple loop works)

The orchestrator holds a pool of Unity players and virtual Pis and
cycles through them for throughput. To leave room for that, the simple
loop reaches every part by an address it is given, never by assuming a
single instance. Known consequence: renderers and detectors keep no
state between frames and pool freely; physics and the virtual Arduino
hold one episode's state; the policy needs its 6-frame window, so a
pooled policy must be sent the window or a loop must stay pinned to one
virtual Pi — decided when pooling arrives.

## References

- `DATASETS.md` — image spec (1456×1088 native, 728×544 inference,
  greyscale, q90)
- `AGENTS.md` — class-id contract, `StateServer` protocol
- greyhound `mujoco-win.md` — MuJoCo 3.15.0 on Windows
