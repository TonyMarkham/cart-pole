# Synthetic Orin Plan — the virtual robot computer (parts 3–4 of `loop-plan.md`)

The policy has to learn from what the robot's computer will see and
remember, so the loop needs a stand-in for that computer: a **synthetic
Orin** that receives each rendered frame, detects the four markers
exactly as the real Orin will, keeps the last six detections as the
policy's timestamped input window, and answers the orchestrator the way
the real Orin answers the Arduino. This plan builds its first part —
frame in, observation out, window kept — with the policy slot still
empty. It is a Python process (detection on the robot is Python, user
2026-10-23) running TensorRT on this PC's RTX 3090.

Status (2026-10-23): **steps 1–6 done, Done-when 1–4 met on this PC**
(see *Results*; uncommitted). R1 approved by the user 2026-10-23.

## Done when

1. **The detector matches the reference.** The synthetic Orin's
   detector, on a TensorRT FP16 engine built for the 3090 from
   `best.onnx`, agrees with Ultralytics' predictor on
   `target/mount2-render-vel/` (250 frames) within `detector-plan.md`'s
   Done-when 2 tolerances: the same detected / not-detected decision for
   every slot, centers within 1 px, confidences within 0.01.
2. **In the loop.** `cargo run --release -p orchestrator -- --player … --physics --orin …`
   runs a whole episode in lockstep: every rendered frame goes to the
   synthetic Orin, which writes one observation line per frame to
   `observations.jsonl` in the run's `--out` folder, and every frame of
   the velocity sine run (`--control sine:0.4,4 --frames 250`) has End
   detected.
3. **The window is right.** The log shows the episode's first
   observation filling all six window entries at ages 0, −T, …, −5T,
   and every later observation pushing the oldest entry out, with ages
   = capture time − newest capture time.
4. **Truth and observation join.** `observations.jsonl` and
   `states.csv` join on the frame id, so the detected positions can be
   compared with the true `qpos` frame by frame (bring-up stage 2).

## Facts this plan builds on

- **Today's turn** (`orchestrator/src/main.rs`, `unity-player-plan.md`
  *Settled decisions*): the orchestrator is the WebSocket server; the
  Unity player connects as a client (`-orchestrator ws://…`). Per frame
  the orchestrator sends one text message `{"id", "joints"}` and the
  player answers with one binary message, the 728×544 greyscale JPEG —
  lockstep, one reply per request, in order. In physics mode the
  orchestrator then logs `id,time,ctrl,cart,pole,command` to
  `states.csv` — `ctrl` the scripted speed (m/s), `command` the
  pulse-rate packet it becomes — and steps MuJoCo `--frame-interval-ms`
  (default 20, a placeholder for T) of simulated time, with the
  synthetic Arduino (`firmware` crate, `loop-plan.md` part 5) turning
  the command into each 2 ms step's `ctrl`. (When this plan was
  written, `--detect` ran the Rust `object-detection` crate on each
  frame in-process; step 6 removed it.)
- **The marker frame** (`loop-plan.md`): 4 slots in class order (0 Left,
  1 Right, 2 Pivot, 3 End), each the box center `(x, y)` normalized to
  [0, 1] of the frame, or `(−1, −1)` when not detected.
- **The policy input** (`loop-plan.md`): the last **6** marker frames,
  each prefixed by its **age** — seconds of simulated time before the
  newest (newest 0, older negative): 6 × (1 + 8) = 54 floats.
- **MuJoCo's clock is the authority** (`loop-plan.md`): every time in
  the loop is simulated time, `data.time`.
- **The detector this plan reuses** (`detector-plan.md` *Results*,
  2026-10-23): on the Orin, Python with OpenCV pre-processing and a
  TensorRT engine built from `best.onnx` (`trt_detect.py`, scratch)
  matched the reference on all 250 frames — FP16 centers within 0.08 /
  0.27 px, confidences within 0.0011 — at 8.4–14.5 ms per frame.
  Pre-processing: OpenCV greyscale decode, Ultralytics letterbox (640,
  pad 114, `round(d/2 ∓ 0.1)` split, `INTER_LINEAR`), ÷255; output
  8 × 8400, box rows in input pixels; slot rule D3 (best candidate per
  class, detected at score ≥ 0.25, center mapped back through the
  letterbox).
- **This PC:** NVIDIA GeForce RTX 3090, 24 GB, driver 616.56. The
  training env's `onnxruntime-gpu` CUDA provider failed to load on
  2026-10-22 (fell back to the CPU); no TensorRT is installed.

## Settled decisions

- **The frame reaches the synthetic Orin through the orchestrator**
  (user, 2026-10-23), as `loop-plan.md` *Transport* has it ("Sim: every
  link goes through the orchestrator"). The Unity player is unchanged:
  it still renders and returns the JPEG. The orchestrator attaches the
  frame's simulated capture time when it forwards the frame.
- **The detector runs TensorRT on the 3090** (user, 2026-10-23): the
  same Python + TensorRT code as the real Orin, with the engine built
  for the 3090.
- **The observation stays on the Orin** (user, 2026-10-23: in
  production it stays there and feeds the decision engine). The
  synthetic Orin keeps it in its window and logs it; its reply to the
  orchestrator is what the real Orin sends out — a command, once a
  policy exists. Until then the reply is an acknowledgement and the
  orchestrator keeps driving the cart with its scripted command.
- **The window** is a `collections.deque(maxlen=6)` of marker frames
  (capture time + 4 slots): appending the 7th drops the oldest.
- **Episode start** (user, 2026-10-23): the first observation of an
  episode fills all six entries — the rig is at rest at the start
  (MuJoCo's reset zeroes velocities), so that history is true. Their
  times are spaced one frame interval apart (capture time, −T, …, −5T)
  so the positions' differences read zero speed and zero acceleration,
  not 0 / 0.
- **`--detect` is superseded** by `--orin`: in-process tract detection
  ran ~110 ms per frame on this PC and is no longer the robot's
  detector. It is removed once Done-when 2 passes (step 6), so the loop
  never lacks a detector in between.

## Messages (orchestrator ↔ synthetic Orin)

The orchestrator stays the WebSocket server; the synthetic Orin connects
as a client, like the player, so each part is reached by an address it
is given (`loop-plan.md` *Later — scaling*).

- **Episode start**, orchestrator → Orin, text:
  `{"type": "reset", "frame_interval": 0.02}` — clears the window; T in
  seconds of simulated time, for the start fill.
- **Frame**, orchestrator → Orin: a text message
  `{"type": "frame", "id": 7, "time": 0.14}` followed by one binary
  message, the JPEG.
- **Reply**, Orin → orchestrator, text, exactly one per frame, in order:
  `{"id": 7}` now; `{"id": 7, "command": …}` once a policy exists.
- **Observation log**, one JSON line per frame in
  `<out>/observations.jsonl`:
  `{"id": 7, "time": 0.14, "slots": [[x, y], [x, y], [x, y], [x, y]], "confidences": [c, c, c, c], "detect_ms": 9.1}`
  — `confidences` and `detect_ms` are for checking, not part of the
  marker frame. The six-entry window (as `[[age, x0, y0, …, x3, y3], …]`)
  is logged on the same line for Done-when 3, **oldest first**: the last
  entry is this frame (age 0), the first is five intervals back (−5T).
  Ages carry MuJoCo's floating-point clock, so −0.1 may read
  −0.09999999999998899.

## Requirements and recommendations

Requirements (the observations are wrong without them):

- **O1 — the detector is the reference pipeline.** OpenCV greyscale
  decode, Ultralytics letterbox, ÷255, slot rule D3 — as in *Facts*;
  Done-when 1 checks it.
- **O2 — times are simulated time.** The observation's `time` is the
  frame's capture time from the orchestrator (MuJoCo's `data.time`),
  never a wall clock.
- **O3 — "not detected" only means not detected.** A frame that does
  not decode, or is not 728×544, is an error reported to the
  orchestrator, which fails the run — never empty slots
  (`detector-plan.md` D4).
- **O4 — lockstep.** One reply per frame, in order; the orchestrator
  does not step MuJoCo until it has the reply.

Recommendations (choices, with the reason):

- **R1 — dependencies, in their own uv project `orin/`** at the repo
  root (Python 3.12, `uv.lock` committed): `tensorrt-cu12` (NVIDIA's
  TensorRT wheels for Windows/Linux; matching CUDA runtime comes with
  them — unverified on this PC until step 1), `cuda-python` (device
  buffers, as on the Orin), `opencv-python-headless`, `numpy`,
  `websockets` (the client). **Needs the user's OK** (installs).
  The same project is what the real Orin will run; only the frame
  source differs (camera vs orchestrator).
- **R2 — the engine is built by a script in `orin/`** with TensorRT's
  Python builder from `best.onnx` (FP16, the robot's precision), saved
  beside `best.onnx` (gitignored) — the Windows wheels ship no
  `trtexec`. An engine is specific to its GPU and TensorRT version, so
  each machine builds its own.
- **R3 — the detector's settings come from `ObjectDetection.toml`**
  (input size, pad value, confidence, classes; Python's `tomllib`),
  with a new key for the engine path — one place for the detector's
  parameters, as the user set for the crate. The Rust loader learns the
  key so its strict parse keeps passing.
- **R4 — the orchestrator launches the synthetic Orin** (`--orin
  <command>`), as it launches the player, and passes it `--orchestrator
  ws://…` and the run's `--out` folder.

## Steps

1. **Environment.** After the OK on R1: `orin/` uv project; check that
   TensorRT loads and sees the 3090. *Check:* a one-line script prints
   TensorRT's version and the GPU name.
2. **Engine.** `orin/build_engine.py` builds the FP16 engine from
   `best.onnx`. *Check:* it loads, and its tensors are input
   `[1, 1, 640, 640]`, output `[1, 8, 8400]`.
3. **Detector** (O1, O3): `orin/` detector module — decode, letterbox,
   TensorRT, slot rule. *Check:* Done-when 1 on `mount2-render-vel`,
   with the time per frame printed.
4. **Synthetic Orin process** (O2, O4, *Messages*): WebSocket client,
   reset handling, window with the start fill, observation log, reply.
   *Check:* a stub run against a recorded frame set.
5. **Orchestrator `--orin`** (R4): launch, accept the second client,
   send `reset` at episode start, forward each frame with its id and
   time, wait for the reply before stepping. *Check:* Done-when 2–4.
6. **Remove `--detect`** from the orchestrator once step 5 passes.

## Not in this plan

The policy and its command (the reply's `command` field), the virtual
Arduino, the delay D, episode end conditions, several episodes per run,
the real Orin's camera capture, and what becomes of the Rust
`object-detection` crate (superseded as the loop's and the robot's
detector; its removal is a separate decision).

## Placeholders

- **T = 20 ms** (`--frame-interval-ms`) until the Orin's frame interval
  is measured; the start fill uses whatever T the run uses.
- **Confidence 0.25** (`ObjectDetection.toml`), as in `detector-plan.md`.

## Results (2026-10-23, executed on this PC)

- **Step 1 — gate passed.** `orin/` uv project, Python 3.12.14 (uv
  managed). The first resolve took `tensorrt-cu12` 11.4.0.106: it loads
  and sees the 3090 (compute capability 8.6), but TensorRT 11 has no
  `BuilderFlag.FP16` — every network is strongly typed, so FP16 needs an
  FP16 ONNX, a different build from the Orin's TensorRT 10.3 (`trtexec
  --fp16`). Capped `tensorrt-cu12>=10,<11` to keep the Orin's builder
  API: **TensorRT 10.16.1.11**, `cuda-python` 12.9.10 (`<13`, CUDA 12 as
  on the Orin), `opencv-python-headless` 5.0.0.93, `numpy` 2.5.4,
  `websockets` 17.3. CUDA is reached as `cuda.bindings.runtime`, which
  the Orin's `cuda-python` 12.6.2.post1 also provides.
- **Step 2.** `orin/build_engine.py <ObjectDetection.toml> <engine>`:
  FP16 builder flag, 235 s on the 3090, 8.5 MB; loads back with input
  `images [1, 1, 640, 640]` and output `output0 [1, 8, 8400]`, float32.
  Engine at `training/runs/cp2-640-base/weights/best_fp16.engine`
  (gitignored).
- **Step 3 — Done-when 1 met.** `orin/detector.py` (`Detector`,
  `MarkerFrame`, `FrameError`), `orin/detector_config.py` (reads
  `model`, `input_size`, `pad_value`, `confidence`, `classes` from
  `ObjectDetection.toml`). `orin/reference_check.py` on
  `mount2-render-vel` (250 frames, 1,000 slots) against
  `target/handoff-scratch/reference/mount2-render-vel.csv`, three runs:
  every decision the same, centers within 0.055 / 0.308 px,
  confidences within 0.0007, no slot near the threshold; **2.7 ms per
  frame** (decode to slots, after a 20-frame warm-up).
- **R3 is half done:** the engine path is a command-line argument
  (`--engine`), not an `ObjectDetection.toml` key. The key would edit
  `ObjectDetection.toml` and the crate's `config.rs`, which carry the
  uncommitted tract/Coral changes whose fate is the user's call.
- **Step 4.** `orin/synthetic_orin.py` (*Messages*, O2–O4). Stub
  orchestrator (scratch) over `mount2-render-vel` with a reset before
  frames 0 and 125 and one truncated JPEG: 250 replies in order, the
  truncated frame answered `{"id": 999, "error": "frame does not decode
  as an image"}`, start fill at both resets, every other window the
  previous one shifted by one.
- **Step 5 — Done-when 2–4 met.** `--orin <command…>` (last on the
  command line; the orchestrator appends `--orchestrator`, `--out`);
  launched after the player connects, `reset` before frame 0, each
  frame forwarded with its capture time, MuJoCo stepped after the
  reply. `cargo run --release -p orchestrator -- --player
  unity/projects/cart-pole/Build/SimulationLoop.exe --physics --control
  sine:0.4,4 --frames 250 --out target/orin-loop-vel --orin uv run
  --project orin python orin/synthetic_orin.py --config
  ObjectDetection.toml --engine
  training/runs/cp2-640-base/weights/best_fp16.engine`: 250
  observations, all four slots detected in every frame; frame 0's
  window ages −0.1 … 0 with one slot set, every later window pushes
  the oldest out; `observations.jsonl` joins `states.csv` on all 250
  ids with identical times. Detection 5.0 ms mean in the loop (frame
  0 50 ms, CUDA's first call; worst after it 8.6 ms); a frame's
  forward-and-ack ~5–7 ms. Found on the way: the Orin's client waits
  for the server to end TCP after the close handshake (up to 10 s), so
  the orchestrator drops the socket before awaiting the exit.
- **Step 6.** `--detect` removed: the flag, its `states.csv` columns,
  and the orchestrator's `object-detection` dependency (the crate
  itself and its workspace entry stay). Done-when 2–4 re-run after the
  removal: the same results.

## References

- `loop-plan.md` — parts, the marker frame, the policy input, timing,
  transport
- `detector-plan.md` — the reference pipeline, D3/D4, *Results* (the
  Orin TensorRT test)
- `unity-player-plan.md` — the player protocol and the orchestrator stub
- `physics-plan.md` — `states.csv` and the physics mode
