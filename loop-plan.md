# Loop Plan — the simulated robot loop (coarse master plan)

The balance policy has to learn from what the real robot will see —
camera frames, not true joint state — so it trains in a loop that runs
the robot's own pipeline against a simulated rig: MuJoCo moves the
cart-pole, Unity renders the camera frame, the robot computer's code
detects the markers, the trainer's policy decides, the Arduino's code
turns the decision into motor steps, and those steps move the MuJoCo
cart. This document fixes the parts, their inputs and outputs, and how
one observation of the loop runs. Each part to be built gets its own
implementation plan; this plan only says what each part must do.

The sequence of one run is drawn in `synthetic-loop-dia.md`, the source
of truth for this plan (2026-10-23): where the two disagree, the diagram
is right and this plan is out of date.

Status: **plan** (2026-10-23). "Exists" below names what is landed
today; everything else is to be built.

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
- **In production the policy runs on the Orin; in the synthetic loop it
  lives in the trainer.** Everything in the synthetic loop runs on this
  workstation, and the trainer is where the policy is trained.
- **The trainer owns the run.** It tells the orchestrator to start, and
  is told when the run has closed. It receives only the marker pool, the
  current state; it generates the reward or penalty and decides
  continue or stop itself. Nothing in the control loop touches training.
  Stop ends this run, one episode for now; the trainer itself carries on
  and starts the next run. A failed run (3 failed frames in the pool) is
  different: it ends training, and the trainer starts no further run.
- **The robot sees only pixels.** The synthetic Orin receives the pose
  only to pass it to Unity at its sim edge; detection sees only pixels,
  and the policy only the marker pool. The true joint state (`qpos`)
  goes only to the orchestrator, for the truth log, never to the
  trainer.
- **MuJoCo's clock is the authority.** Simulated time advances only by
  physics steps, a fixed 2 ms each; every delay in the loop is expressed
  in simulated time.

## The parts

| # | Part | In | Out | Real-robot counterpart |
|---|---|---|---|---|
| 0 | **Orchestrator** | — | the clock (MuJoCo's steps), the Orin's latency, the truth log, process supervision | none (sim only) |
| 1 | **Physics** — MuJoCo + the exported MJCF | actuator control value `ctrl` | true joint positions `qpos` | the physical rig |
| 2 | **Renderer** — Unity player + `StateServer` | joint positions, from the synthetic Orin | 728×544 greyscale JPEG, q90 | Pi GS camera, frame reduced to 728×544 greyscale |
| 3 | **Detector** — virtual Pi (the synthetic Orin) | JPEG | 4-slot marker frame, pushed onto the 6-entry pool | same code on the Pi |
| 4 | **Policy** — inside the trainer | the pool: 6 marker frames, each with its age | signed step-pulse rate | the trained policy, on the Pi |
| 5 | **Virtual Arduino** — firmware, `synthetic` build | signed step-pulse rate | DRV8825 signals, DIR and STEP rate, which the orchestrator turns into `ctrl` in m/s | the Arduino + stepper + belt |
| 6 | **Trainer** — holds the policy being trained | the pool | the command + continue/stop | none (sim only) |

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
- **5 Virtual Arduino** — the `firmware` crate (2026-10-23,
  uncommitted): a `no_std`, integer-only core (command decode, step
  generator: top-rate clamp, fraction carry, travel envelope) and its
  `synthetic` feature, `SyntheticArduino`, linked into the orchestrator
  like the physics crate. Its constants are `firmware::RIG` (one source
  for both builds; the board has no config file). The board's `live`
  edge (radio, step pin, timer) is not written.
- **6 Trainer** — nothing.

## Contracts between the parts

- **Frame (2 → 3):** 728×544 greyscale JPEG, quality 90 — exactly half
  the IMX296's native 1456×1088 (`DATASETS.md`, *Image spec*).
- **Marker frame (3 → pool):** 4 slots in class order — 0 Left (tag
  376), 1 Right (596), 2 Pivot (920), 3 End (937), the repo's class-id
  contract. Each slot is the box center `(x, y)`, normalized to [0, 1],
  or `(-1, -1)` when that class was not detected. `-1` lies outside the
  valid range, so "not detected" can never be a real position. When a
  class has several boxes, the slot takes the highest-confidence one.
  The flag means *not detected*, whatever the cause (occluded, missed,
  out of frame, or a failed frame).
- **Failed frame:** a frame that does not decode, or is not 728×544. Its
  entry is 4 × (−1, −1) and the error is logged; one failed frame costs
  about 15 ms of detection and does not fail the run. Three failed
  frames among the pool's 6 entries fail the run and end training.
  Pre-warm entries are no detection, not failed frames.
- **Policy input (pool → 6):** the synthetic Orin's pool, the last
  **N = 6** marker frames, newest first, each keyed by its **age** —
  milliseconds before the current capture time (newest = 0, older
  negative). 6 × (1 + 8) = **54 floats**. Ages are relative so the
  input never depends on absolute clock values, and the policy sees the
  real spacing, which varies with the Orin's latency. Three entries are
  the minimum for the current position, speed and acceleration; the
  other three are redundancy for detections that drop out. At startup
  the Orin pre-warms the pool with six entries of no detection
  (−1, −1), at seeded gaps before the first capture.
- **Command (6 → 5):** a signed **step-pulse rate** — sign = direction,
  magnitude = the target pulses per unit time the firmware emits to the
  stepper driver; never a distance or position offset (user,
  2026-10-23). The packet is **4 bytes, one `i32`, little-endian, in
  pulses per second**, fixed length, no id (user, 2026-10-23). Why 4:
  at 1/32 microstepping (the DRV8825's finest) the ±0.68 m/s
  `ctrlrange` is 0.68 / 0.096 × 200 × 32 ≈ 45,200 pulses/s, past an
  `i16`'s 32,767; an `i32` never limits the microstep setting or the
  speed. One pulse per second is 0.015 mm/s at 1/32, finer than the
  slowest motion a balance needs. The trainer's continue/stop travels
  with the command to the orchestrator; only the command reaches the
  Arduino.
- **Actuator value (5 → 1):** once per observation, at the arrival step
  (*Timing*), the orchestrator hands the command to the synthetic
  Arduino, which clamps it to the motor's top rate (±45,333 pulses/s)
  and returns the DRV8825 signals: DIR from the sign, STEP rate from
  the magnitude. The orchestrator sets MuJoCo's `<velocity>` actuator
  to that rate as a cart speed: `ctrl` = STEP rate × meters per pulse,
  in m/s, signed by DIR. Meters per pulse = belt travel per pulley
  revolution ÷ (motor full steps per revolution × driver microstep
  setting): 96 mm ÷ (200 × 32) = 15 µm at `RIG`. The belt travel is
  96 mm: the CAD pulley `pully-gt2-48t-6w-8b` has 48 teeth at GT2's
  2 mm pitch. The microstep setting (32 in `RIG`) is a placeholder (see
  *Placeholders*). The MJCF's `ctrlrange` (±0.68 m/s) matches the
  firmware's top rate, 45,333 pulses/s × 15 µm. The `live` build sends
  the pulses to the driver's step pin and its timer counts them, so its
  ±30,000-pulse travel limit acts on the board; the `synthetic` build
  receives only the command and keeps no pulse count, so in the loop the
  travel limit cannot act and the end crash is the trainer's training
  failure (*Training settings*).
- **Truth (1 → 0):** `qpos` to the orchestrator only — the truth log,
  never the trainer.
- **Truth log:** `states.csv`, one row per observation: id, capture
  time, the true cart (m) and pole (rad) at capture, and the 4 observed
  `(x, y)` of the pool's newest entry — MuJoCo's real values paired with
  what the Orin observed, for evaluation after training. An early stop
  by the orchestrator on a truth-versus-observation threshold is not in
  the loop yet.

## Timing

- **MuJoCo's `data.time` is the clock.** It advances 2 ms per physics
  step, a fixed deltatime (the MJCF sets no `<option>`, so MuJoCo's
  default timestep applies), and by nothing else.
- **There is no fixed frame interval.** Each capture goes out at the
  previous observation's arrival step; the first at t = 0.
- **The Orin's latency, in simulated time.** When the orchestrator
  takes a capture state, it draws the latency, a seeded RND(8, 15) ms,
  and caches the arrival time = capture time + latency. MuJoCo keeps
  stepping on the old command while the Orin works, as the real Arduino
  keeps stepping until the next packet lands. The observation reaches
  the trainer at the first step whose time is later than the arrival
  time, the arrival step; the trainer's command takes effect there, and
  the next capture goes out at that same step. Captures sit on the 2 ms
  grid, so the arrival step rounds the latency up to the next step: the
  gap between two captures is 10, 12, 14 or 16 ms. The range is the Orin's
  measured detection time, 8.4–14.5 ms over 250 frames (`orin-plan.md`,
  *Facts*), a placeholder for D = detection + policy + WiFi; drawing it
  at random is the band the policy trains in, because the policy cannot
  observe it directly.
- **Wall clock: the Orin's round trip.** MuJoCo runs far faster than
  real time, so it usually reaches the arrival step before the Orin has
  replied; the orchestrator then freezes it there until the reply is in.
  The run goes as fast as the Orin's round trip allows, and the
  workstation's speed never changes simulated time.

## One observation of the loop (n)

`synthetic-loop-dia.md` draws it message by message.

1. **Capture.** The orchestrator takes MuJoCo's current `t`, `qpos`
   (the start state, then each arrival step's) and draws the latency.
2. **Observe, while physics steps.** The orchestrator sends the pose to
   the synthetic Orin, which triggers Unity; the Orin detects, pushes the
   marker frame onto the pool, and replies with the pool, no timing.
   Meanwhile MuJoCo steps every 2 ms on the old command up to the
   arrival step, and is frozen there until the reply is in.
3. **A failed run** (3 failed frames in the pool) goes to the shutdown,
   and training ends.
4. **Truth log.** The orchestrator writes the observation's
   `states.csv` row.
5. **Trainer.** It receives the pool only; it scores its last command
   from it (this pool is the next state after it), decides continue or
   stop, and returns the next command.
6. **Continue:** the Arduino turns the command into DIR and STEP rate,
   and the orchestrator sets `ctrl` at the arrival step, which is also
   the next capture. **Stop:** the shutdown, then the trainer is told
   the run has closed.

## Transport

- **Sim:** the trainer and the synthetic Orin talk to the orchestrator;
  Unity talks only to the synthetic Orin, which passes it the pose and
  returns only the pool to the orchestrator. Only one message to
  another process is in flight at any moment.
- **Real:** the Pi sends command packets to the Arduino over WiFi.
- The messages are drawn in `synthetic-loop-dia.md`; wire formats
  beyond it belong to the per-target implementation plans.

## Targets — each gets its own implementation plan

1. **Orchestrator** — the clock (MuJoCo's fixed step, the Orin's
   latency, the freeze at the arrival step), the truth log, process
   supervision. **The trainer owns the run**
   (`synthetic-loop-dia.md`, 2026-10-23): the trainer tells the
   orchestrator to start and is told when the run has closed; the
   orchestrator feeds it the pool and carries out its command and
   continue/stop.
2. **Physics** — MuJoCo stepping on the exported MJCF, in-process: the
   `physics` crate linked into the orchestrator, not a worker process;
   accepts `ctrl`, reports `qpos`. Plan: `physics-plan.md`.
3. **Unity player** — a standalone build of the renderer at 728×544
   greyscale, q90.
4. **Pi code, `live` + `virtual`** — JPEG decode, detector inference
   and slot filling, the 6-entry pool; the policy runs here in
   production, in the trainer in the synthetic loop. Detection on the
   robot is Python (*Principles*, exception). The object detection
   crate's parameters are configured outside source code, in
   `ObjectDetection.toml` at the repo root (user, 2026-10-22), as
   `physics.toml` configures the physics crate.
5. **Firmware, `live` + `synthetic`** — command decode, top-rate clamp,
   step generation inside the travel envelope on the board; the
   `synthetic` edge returns DIR and STEP rate, which the orchestrator
   turns into `ctrl`. The `synthetic` side exists (`firmware/`, part 5
   above); `live` is to be written.
6. **Trainer** — holds the policy; owns the run; the reward or penalty,
   continue or stop, and the end crash's bounds.

## Bring-up — what fills the policy slot

Each stage must pass before the next, so a failure has one place to look.

1. **Zero command** — the trainer returns a zero command; the cart holds
   still, the pole hangs or swings. Proves the plumbing: every part
   runs, frames return, the Pivot/End slots track the swinging pole, the
   run starts and closes.
2. **Scripted command** — the trainer returns a slow back-and-forth.
   Proves the command path (`i32` → firmware → `ctrl` → physics →
   render) and lets the detector's slots be compared with the true
   positions in `states.csv`.
3. **Trained policy** — only after 1 and 2 pass; a failure here is
   learning, not plumbing.

## Training settings (tuned during training, not loop structure)

- **Episode start state:** the cart always starts at 50% of the rail, a
  guarantee that stands in for the real board's homing; the pole's start
  (near θ = π) is a training setting.
- **End conditions and the reward** are the trainer's. One end
  condition is the end crash: the Pivot marker out of bounds is a
  training failure, penalty, and stop this run. The bounds must sit inside the
  slide's ends by more than the cart can travel between a capture and
  the stop taking effect, at most 0.68 m/s × 16 ms ≈ 11 mm
  (`synthetic-loop-dia.md`, *Numbers*).
- **One episode per run** for now; more than one is not yet in the loop.

## Placeholders and checks to close

- **The Orin's latency** — RND(8, 15) ms stands in for D = detection +
  policy + WiFi, to be measured on the Orin.
- **The end crash's bounds** — not set.
- **Pi image reduction** — the sensor is colour; the Pi's reduction to
  728×544 greyscale must use the same method as the renderer's, or the
  real frames will not look like the training frames.
- **GPU vs CPU inference** — the same ONNX model on the 3090 and on the
  Pi can differ slightly; compare once.
- **Meters per microstep** — the driver's microstep setting
  (`firmware::RIG.microsteps` = 32, the DRV8825's finest, until chosen);
  the motor's 200 full steps per revolution are from third-party
  23HS32-4004S listings.

## Later — scaling (only after the simple loop works)

The orchestrator holds a pool of Unity players and virtual Pis and
cycles through them for throughput. To leave room for that, the simple
loop reaches every part by an address it is given, never by assuming a
single instance. Known consequence: renderers and detectors keep no
state between frames and pool freely; physics and the virtual Arduino
hold one episode's state; the synthetic Orin keeps the 6-entry pool, so
a loop must stay pinned to one synthetic Orin, or the pool must move
with the loop — decided when pooling arrives.

## References

- `synthetic-loop-dia.md` — the sequence of one run, the source of
  truth for this plan
- `DATASETS.md` — image spec (1456×1088 native, 728×544 inference,
  greyscale, q90)
- `AGENTS.md` — class-id contract, `StateServer` protocol
- greyhound `mujoco-win.md` — MuJoCo 3.15.0 on Windows
