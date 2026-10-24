# Detector Plan — the virtual Pi's object detection (part 3 of `loop-plan.md`)

The policy must act on what the real robot sees, so every rendered frame
has to be turned into exactly what the Pi will compute from its camera:
the **4-slot marker frame** — where each of the four markers is, or that
it was not detected. This plan builds that step as a Rust library crate,
`object-detection`, that runs the trained `best.onnx` on a JPEG and
returns the slots; one source serves the loop (`virtual`, called by the
orchestrator) and the Pi (`live`), so it must build on the robot's
computer, where the repo is cloned and compiled for production. Its
parameters live in `ObjectDetection.toml` at the repo root.

Status (2026-10-23): **steps 1–7 done, Done-when 1–4 met on the
Raspberry Pi 5** (see *Results*). **The target is now the Jetson Orin
Nano Super** (user, 2026-10-23, after the measurements in *Results*):
TensorRT FP16 on its GPU detects in 8–10 ms per frame at float
accuracy, against 80–125 ms for the best Pi 5 options. **Detection on
the robot runs in Python** (user, 2026-10-23): OpenCV pre-processing +
TensorRT, 11.9–14.5 ms per frame against the Rust POC's 18.7–19.1 ms
in the same session. That Python path exists only as the scratch test
(`trt_detect.py`). The crate keeps its engines (tract on the CPU,
which the loop on the PC uses; the `edgetpu` Coral engine,
uncommitted).

**The planned loop is `synthetic-loop-dia.md`** (2026-10-23), the
source of truth: it supersedes the decisions marked *Superseded* below.
In it, detection runs in the synthetic Orin (Python + TensorRT,
`orin-plan.md`), not in this crate called by the orchestrator. What this
plan built and measured (*Results*) stays as the record.

## Done when

1. `cargo build -p object-detection` builds with pure-Rust dependencies,
   and the crate loads `best.onnx` with the input and output shapes in
   *Facts* (step 2).
2. **Matches the reference.** On the loop frames rendered at the mount
   pose (`target/mount2-sweep/`, 60 frames; `target/mount2-render-vel/`,
   250 frames), the crate's slots agree with Ultralytics' own predictor
   on the same `best.onnx` (the reference run of 2026-10-22,
   `unity-player-plan.md` *Next* 2): the same detected / not-detected
   decision for every slot, centers within 1 px (0.0014 of the width,
   0.0018 of the height), and confidences within 0.01. A slot whose
   reference confidence lies within 0.01 of the threshold may differ in
   its decision, and is listed rather than failed.
3. **In the loop.** `cargo run -p orchestrator -- --physics --detect …`
   writes each frame's four slots into `states.csv` beside the true
   `qpos`, and the run's End slot is detected in every frame of the
   velocity sine run (as the reference was).
4. **Builds on the Pi 5** in a clone of the repo there (step 7).

## Facts this plan builds on

- **The model** `training/runs/cp2-640-base/weights/best.onnx` (read with
  onnxruntime in the training env, 2026-10-22). It is gitignored
  (`.gitignore:20`, `runs/`), so the config names its path; the repo
  does not carry it.
  - input `images`, `[1, 1, 640, 640]`, float32 — one greyscale channel;
  - output `output0`, `[1, 8, 8400]`, float32 — 8400 candidates; rows
    0–3 are the box center x, center y, width, height in the 640×640
    input's pixels, rows 4–7 the four class scores;
  - no NMS in the graph (`end2end = False`, `nms = None`);
  - metadata: `channels = 1`, `imgsz = [640, 640]`, `stride = 32`,
    `names = {0: Left, 1: Right, 2: Pivot, 3: End}`, Ultralytics 8.4.177,
    license AGPL-3.0.
- **Ultralytics' pre-processing for this model** (`ultralytics`
  8.4.177 in the training env: `engine/predictor.py`,
  `data/augment.py` `LetterBox`, `data/loaders.py`):
  - the frame is read greyscale (`cv2.IMREAD_GRAYSCALE`, because
    `channels = 1`); the loop's frames are greyscale stored as three
    equal channels, so any one channel is the grey value;
  - letterbox to the full 640×640 (`auto` is off for a static ONNX):
    r = min(640 / h, 640 / w); resized size = (round(w·r), round(h·r));
    the padding is split `top = round(dh/2 − 0.1)`,
    `bottom = round(dh/2 + 0.1)` (same for left/right) and filled
    with 114; the resize is OpenCV `INTER_LINEAR`;
  - pixel values are divided by 255.
  For a 728×544 frame: r = 640/728 = 0.879121, resized 640×478,
  81 rows of padding top and bottom, none left or right.
- **The marker-frame contract** (`loop-plan.md`): 4 slots in class order
  (0 Left / tag 376, 1 Right / 596, 2 Pivot / 920, 3 End / 937), each the
  box center `(x, y)` normalized to [0, 1] of the original frame, or
  `(−1, −1)` when that class was not detected; with several boxes of a
  class, the highest-confidence one. An obscured marker is not
  registered, by intent.
- **The reference run** (`unity-player-plan.md`, *Next* 2): Ultralytics'
  predictor at `conf` 0.25 on the mount-pose frames found End in every
  frame and all four markers wherever none is obscured; the per-frame
  reference CSVs and the script that wrote them are scratch files, to be
  regenerated for step 5.
- **Requirements already set by the user:** the Pi's code is built on
  the Pi 5, aarch64 Linux (`loop-plan.md`, *Principles*); the object
  detection crate is configured by `ObjectDetection.toml` at the repo
  root (`loop-plan.md`, *Targets* 4).
- **Precedent:** the `physics` crate and `physics.toml` (`physics-plan.md`)
  — workspace member, config file at the repo root with paths relative
  to the file's folder, strict parse (`deny_unknown_fields`).

## Settled decisions

- **A workspace library crate `object-detection`** (the name follows
  `ObjectDetection.toml`): JPEG bytes in, the 4-slot marker frame out.
  No I/O beyond reading its config and model; the caller supplies the
  frame — the orchestrator in the loop, the camera loop on the Pi later.
- **`ObjectDetection.toml` at the repo root** configures it (user,
  2026-10-22).
- **Every dependency builds for aarch64 Linux** (user, 2026-10-22).

## `ObjectDetection.toml`

```toml
# Configuration of the object-detection crate (detector-plan.md).
# Relative paths resolve against this file's folder.

# The trained detector (gitignored; produced by training/).
model = "training/runs/cp2-640-base/weights/best.onnx"

# The model's square input size, pixels (its metadata imgsz).
input_size = 640

# Ultralytics' letterbox padding value (0-255 grey).
pad_value = 114

# A class's best box counts as detected at or above this score.
# 0.25 is Ultralytics' predict default, the value the reference used.
confidence = 0.25

# Class order = slot order = the model's class ids (its metadata names).
classes = ["Left", "Right", "Pivot", "End"]
```

Rules:

- **The path is given at run time**, not baked in at build time as
  `physics.toml`'s is. The library takes the file's path; the orchestrator passes the repo-root file, and
  the Pi's binary will take it as an argument (its default is that
  leg's decision).
- **Checked at load:** the model's input and output shapes must be
  `[1, 1, input_size, input_size]` and `[1, 4 + classes, N]`; any other
  model fails the load instead of misreading rows.

## Requirements and recommendations

Requirements (the slots are wrong without them):

- **D1 — Ultralytics' letterbox, exactly.** The scale, the rounded
  resized size, the `round(d/2 ∓ 0.1)` padding split, pad value 114, and
  ÷255, as in *Facts*. A different scale or offset shifts every box.
- **D2 — bilinear resize the way OpenCV does it** (`INTER_LINEAR`:
  source coordinate `(dst + 0.5) / r − 0.5`, clamped at the edges, the
  four neighbours weighted linearly). A different filter changes pixel
  values, and with them confidences near the threshold.
- **D3 — slot rule.** For each class, the candidate with the highest
  score; detected when that score ≥ `confidence`. Its center maps back
  through the letterbox — `x = (cx − left) / r`, `y = (cy − top) / r` —
  and is normalized by the frame's width and height. Otherwise
  `(−1, −1)`. No NMS: one best box per class is the contract.
- **D4 — frame checks.** A frame that is not a JPEG, or whose size is
  not the one the slots are normalized to, is an error, not empty slots
  — "not detected" must only ever mean the model found nothing.
  *Superseded (`synthetic-loop-dia.md`):* in the loop such a frame is a
  failed frame, its entry 4 × (−1, −1) and the error logged; one does
  not fail the run, but 3 among the pool's 6 entries fail it and end
  training.

Recommendations (choices, with the reason):

- **R1 — `tract-onnx` for inference** (latest 0.23.9 on crates.io,
  2026-10-22). Pure Rust: it builds on aarch64 like any crate, and the
  Pi binary carries the engine, nothing else to install there. `ort`
  (ONNX Runtime) is faster and GPU-capable but links a native library
  that would have to be built for aarch64 and installed on the Pi.
  Unverified until step 2: that tract runs this YOLO11n graph and
  matches onnxruntime's output. **Needs the user's OK** (a new crate).
- **R2 — `image` with only its JPEG decoder** (`image` 0.25.10,
  `default-features = false`, `features = ["jpeg"]`). Pure Rust. Its
  decoder can differ from OpenCV's libjpeg-turbo by a grey level here
  and there; step 5's tolerances are where that shows. **Needs the
  user's OK.**
- **R3 — the resize is written by hand** (D2, ~30 lines) rather than
  taken from `image`'s filters, whose downscaling is not OpenCV's
  bilinear; the formula is checkable by hand against D2.
- **R4 — the reference comparison stays out of the repo's runtime
  path.** The Ultralytics reference is a Python run in the training env
  (interactive tooling, where Python is allowed); its per-frame CSV is
  the input to a Rust comparison test. No Python is added to the
  orchestrator or the crate.

## Steps

1. **Workspace and config.** After the user's OK on R1/R2: `tract-onnx`
   and `image` added to `[workspace.dependencies]`; new member
   `object-detection/` (library; workspace version, edition, lints);
   `ObjectDetection.toml` at the repo root as above;
   `DetectorConfig::load(path)` — strict parse, `model` resolved against
   the file's folder. *Check:* `cargo build -p object-detection`.
2. **Load the model.** tract loads `best.onnx`, fixes the input fact to
   `[1, 1, 640, 640]` f32, optimizes, and checks the output shape
   (`[1, 8, 8400]`). *Check:* one frame's raw output agrees with
   onnxruntime's on the same pre-processed tensor (max absolute
   difference printed; expected ≲ 1e-4). This is the first real risk:
   if tract cannot run the graph, the plan stops here and R1 is
   revisited with the user.
3. **Pre-process** (D1, D2): decode → one grey channel → letterbox →
   ÷255 → the `[1, 1, 640, 640]` tensor. *Check:* for the 728×544 frame,
   r = 0.879121, resized 640×478, 81 rows of padding top and bottom;
   one padded row reads 114/255.
4. **Post-process** (D3) → `MarkerFrame { slots: [(f32, f32); 4] }` plus
   each slot's confidence (for logging and the comparison).
5. **Reference comparison** (Done-when 2): regenerate the Ultralytics
   reference CSVs for `target/mount2-sweep/` and
   `target/mount2-render-vel/` (training env, scratch script); a Rust
   test or small binary runs the crate over the same frames and checks
   the tolerances. Every mismatch is printed with both values.
6. **Orchestrator `--detect`** (Done-when 3): `object-detection` as an
   orchestrator dependency; with `--detect`, each returned frame is run
   through the detector and its slots are appended to the `states.csv`
   row (`left_x,left_y,…,end_x,end_y` plus the four confidences). Per-
   frame detection time is printed (not a contract; the Pi's own timing
   is what matters for T and D).
7. **Build on the Pi** (Done-when 4): the user clones the repo on the
   Pi 5 and runs `cargo build --release -p object-detection` there.

## Not in this plan

The 6-frame window with ages and the policy input (with the policy leg),
the Pi's camera capture and its greyscale reduction (`loop-plan.md`
*Placeholders*), deploying to the Pi, GPU inference, the delay D, and
any retraining.

## Placeholders

- **`confidence` = 0.25** — Ultralytics' default, kept so the reference
  comparison is like for like; tuned later against real frames.
- **The model** — `cp2-640-base`; a retrain changes only the config's
  `model` path (and, if the classes change, `classes`).

## Results (2026-10-22, executed on this PC)

- **Step 1.** `tract-onnx` 0.23.9 and `image` 0.25.10 (`jpeg` only) in
  `[workspace.dependencies]`; crate `object-detection/`;
  `ObjectDetection.toml` as above. `DetectorConfig::load` also requires
  exactly 4 classes (the marker frame's slot count).
- **Step 2 — gate passed.** tract loads `best.onnx`; output
  `[1, 8, 8400]`. On Ultralytics' own pre-processed tensor for 3 frames,
  tract vs onnxruntime 1.26.0 (CPU): scores differ by ≤ 4.1e-6, box rows
  by ≤ 2.8e-3 px (float32 rounding on values up to 640; above the
  plan's 1e-4 guess in absolute terms, irrelevant against 1 px); the
  same best candidate per class.
- **Step 3.** Geometry for 728×544 as predicted: scale 0.879121, 640×478,
  left 0, top 81; padded rows read 114. The hand-written resize is
  **bit-exact** with OpenCV 5.0.0: fed cv2's grey image, 0 of 409,600
  input values differ from Ultralytics' tensor. Correction to D2: OpenCV
  samples each axis at `(d + 0.5) · src/dst − 0.5` with that axis's own
  ratio (544/478 for y), not `1/r`; it works in 11-bit fixed point, which
  the crate reproduces. Decode: `image` vs libjpeg-turbo grey differs by
  1 level in ~0.17% of pixels (R2's expectation), ≤ 541 input values
  per frame.
- **Step 4.** `MarkerFrame { slots, confidences }`; a slot's confidence
  is its class's best score even when below the threshold. D4: `detect`
  takes the frame size the slots are normalized to and fails on any
  other size or on a non-JPEG.
- **Step 5 — Done-when 2 met.** Reference CSVs regenerated with
  HANDOFF's `detect_check.py`; `cargo run --release -p object-detection
  --example reference_check -- ObjectDetection.toml <csv> <frames>`:
  `mount2-sweep` 60 frames / 240 slots, `mount2-render-vel` 250 / 1,000
  slots — every decision the same, centers within 0.078 px x and 0.236
  px y, confidences within 0.0006, no slot near the threshold.
- **Step 6 — Done-when 3 met.** `cargo run --release -p orchestrator --
  --player unity/projects/cart-pole/Build/SimulationLoop.exe --physics
  --control sine:0.4,4 --frames 250 --out target/mount2-detect-vel
  --detect`: all four slots detected in all 250 frames; `states.csv`
  gains `left_x … end_y` and the four confidences. Detection takes
  ~190–230 ms per frame in release on this PC's CPU (single model, not
  a contract) and ~2.1 s in a debug build, so loop runs with `--detect`
  use `--release`.
- **Step 7 — Done-when 4 met** (robo-pi: Raspberry Pi 5 Model B, 4
  cores, Ubuntu 26.04 aarch64, rustc 1.99.1, gcc). In a clone at
  `8fb7d420`, `cargo build --release -p object-detection --example
  reference_check` built in 6 min. `tract-linalg` compiles its ARM
  kernels with `cc`, so the Pi needs gcc beside the Rust toolchain.
  `reference_check` on `mount2-render-vel` (250 frames): the same
  agreement as on the PC (0.078 / 0.236 px, 0.0006), **mean detect
  486 ms** — about 2 frames per second, tract on one thread.
- **Speed (PC, 2026-10-22, scratch POC, not the crate).** onnxruntime
  1.26 on the CPU with the same model and pre-processing, matching the
  reference on all 310 frames: 65 ms inference on 1 thread, 22 ms on 4,
  against tract's ~192 ms on 1 thread. tract is ~3× slower per thread
  here and runs single-threaded (its `multithread-mm` feature is off).
- **tract on 4 threads** (user-approved `tract-linalg` 0.23.9 with
  `multithread-mm`; `threads = 4` in `ObjectDetection.toml`;
  uncommitted, applied on the Pi as a patch): same agreement on
  `mount2-render-vel`; mean detect PC 188 → 112 ms, **Pi 486 → 377 ms**
  (1.3×, about 2.7 frames per second).
- **Coral USB Edge TPU on the Pi** (scratch POC, 2026-10-23, not the
  crate): `best.pt` exported in WSL with Ultralytics 8.4.177
  `format=edgetpu` (int8, calibrated on the 1,000 `run_Set-11` frames;
  Edge TPU compiler 16.0). The compiler maps 134 of 479 ops: YOLO11's
  attention block (`BATCH_MATMUL`, `SOFTMAX`) ends the one subgraph
  the Edge TPU can take, so the rest (174 of 209 convolutions) runs on
  the Pi's CPU. On the Pi (community `libedgetpu1-std`
  16.0tf2.19.1, `ai-edge-litert` 1.3.0, 4 CPU threads, Coral on USB
  3): **82.5 ms per frame (12 fps)** on `mount2-render-vel`; every
  decision the same as the reference, but centers within 4.3 px x /
  3.1 px y and confidences within 0.031 — outside Done-when 2's float
  tolerances. The output tensor is int8 with one scale for every row
  (0.00418): a box coordinate moves in steps of 0.00418 × 640 /
  0.879 ≈ 3.0 frame px.
- **The crate on the Coral** (user decisions 2026-10-23: link Ubuntu's
  `libtensorflow-lite-dev` 2.14.1 on the Pi; the PC keeps tract on the
  float model). Feature `edgetpu`: TFLite C API + `libedgetpu.so.1`,
  hand-written FFI (`edgetpu_ffi.rs`), engine `edgetpu_engine.rs`;
  without it `tract_engine.rs` as before. `ObjectDetection.toml` gains
  `edgetpu_model`. On the Pi, `cargo build --release -p
  object-detection --features edgetpu` links and loads the Coral
  (TFLite 2.14.1 with the TF 2.19.1-built driver works);
  `reference_check` on `mount2-render-vel`: every decision the same,
  centers within 4.3 / 3.6 px, confidences within 0.025 (940 of 1,000
  slots outside the float tolerances, exit 1 by design), **mean detect
  112 ms** — slower than the Python POC's 82.5 ms.
- **Where the edgetpu build's time goes** (Pi, 250 frames, scratch
  stage timer): JPEG decode 2.2 ms, RGB→luma 1.0, letterbox + resize
  2.8, quantize 0.4, dequantize 0.1 — about 6.5 ms of Rust work; the
  rest is TFLite's invoke (Coral plus the CPU ops it could not map):
  ~125 ms at 1 thread, ~113 at 2, ~115 at 3 and 4. The gap to the
  Python POC (LiteRT 1.3.0, ~80 ms including its numpy conversions) is
  in the invoke, i.e. Ubuntu's TFLite 2.14.1 runs the same CPU ops
  slower; why is not measured.
- **NCNN on the Pi's CPU** (scratch POC, 2026-10-23): `best.pt`
  exported in WSL with Ultralytics 8.4.177 `format=ncnn` (PNNX
  20260526, FP32 weights); run with `ncnn` 1.0.20261020 in Python,
  Vulkan off as Ultralytics' backend sets it. On `mount2-render-vel`:
  every decision the same, centers within 0.30 / 0.43 px, confidences
  within 0.0025 — inside Done-when 2's float tolerances. Time per frame
  (decode + letterbox ~2.2 ms included): 4 threads 103–126 ms over six
  runs (run-to-run spread, the default and an explicit 4 the same; Pi
  at 50–58 °C), 2 threads 116, 1 thread 147. Ultralytics' published
  Pi 5 NCNN figures (YOLO26n 67 ms, YOLO11n 88 ms, inference only)
  are faster than measured here.
- **Jetson Orin Nano Super, TensorRT** (scratch POC, 2026-10-23;
  robo-orin: L4T R36.5.2, JetPack 6.2.3 — CUDA 12.6, TensorRT 10.3 —
  power mode 25W, GPU at 918 MHz, 49–51 °C). Engines built from the
  float `best.onnx` with `trtexec`: GPU compute 4.43 ms (FP16) and
  7.75 ms (FP32) per frame. End to end in Python (OpenCV decode +
  letterbox 2.4–4.0 ms, copy, run, copy back) over `mount2-render-vel`:
  **FP16 8.4–10.4 ms (96–120 fps)**, FP32 13.2–16.2 ms (62–76 fps),
  over 3 and 2 runs (the first runs slower while the clocks ramp up).
  Every decision the same as the reference; centers within 0.08 /
  0.27 px (FP16) and 0.08 / 0.29 px (FP32), confidences within
  0.0011 / 0.0007 — inside Done-when 2's float tolerances.
- **Rust on the Orin** (scratch POC `rust-trt`, 2026-10-23): the
  crate's `Grey::decode` + `Letterbox` (commit `8fb7d420`), TensorRT
  through a C++ shim (`enqueueV3`, pageable copies), the D3 slot rule.
  Same engines and frames: **FP16 18.7–19.1 ms (52–53 fps)**, FP32
  28.2–28.3 ms; decode 5.0–5.2, letterbox 3.5–3.7, copy + infer
  10.0–10.3, slot rule 0.08; every decision the same, centers within
  0.08 / 0.26 px. Python in the same session: 11.9–14.5 ms (decode +
  letterbox with OpenCV 3.5–3.9, copy + infer 8.5–10.5). The gap is the
  CPU pre-processing (8.6 ms Rust vs ~3.7 ms OpenCV). Clocks at 25W:
  CPU max 1.344 GHz (`schedutil`); GPU max 918 MHz but the governor
  (`nvhost_podgov`) held it at 408–510 MHz during both runs (GPU load
  ~45–77%), against `trtexec`'s saturated 4.43 ms — so per-frame
  inference here is clock-bound, not engine-bound.

## References

- `synthetic-loop-dia.md` — the planned loop, the source of truth for
  the decisions marked *Superseded*
- `loop-plan.md` — the marker-frame contract, the build-on-the-Pi
  principle, *Targets* 4
- `unity-player-plan.md` *Next* 2 — the reference runs and the mount
  pose
- `physics-plan.md` — the config-file precedent
- Ultralytics 8.4.177 source in `training/.venv`: `engine/predictor.py`
  (`preprocess`, `pre_transform`), `data/augment.py` (`LetterBox`),
  `data/loaders.py` (greyscale read)
