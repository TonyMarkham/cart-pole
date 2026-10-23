# Detector Plan — the virtual Pi's object detection (part 3 of `loop-plan.md`)

The policy must act on what the real robot sees, so every rendered frame
has to be turned into exactly what the Pi will compute from its camera:
the **4-slot marker frame** — where each of the four markers is, or that
it was not detected. This plan builds that step as a Rust library crate,
`object-detection`, that runs the trained `best.onnx` on a JPEG and
returns the slots; one source serves the loop (`virtual`, called by the
orchestrator) and the Pi (`live`), so it must build on the Pi 5, where
the repo is cloned and compiled for production. Its parameters live in
`ObjectDetection.toml` at the repo root.

Status (2026-10-22): **steps 1–6 done, Done-when 1–3 met** (see
*Results*; uncommitted). R1/R2 approved by the user. Step 7 (the build
on the Pi) is the user's, on the Pi.

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
- **Step 7 fact.** `tract-linalg` compiles its hand-written ARM kernels
  with `cc` at build time, so the Pi needs a C compiler beside the Rust
  toolchain. Not yet built on the Pi.

## References

- `loop-plan.md` — the marker-frame contract, the build-on-the-Pi
  principle, *Targets* 4
- `unity-player-plan.md` *Next* 2 — the reference runs and the mount
  pose
- `physics-plan.md` — the config-file precedent
- Ultralytics 8.4.177 source in `training/.venv`: `engine/predictor.py`
  (`preprocess`, `pre_transform`), `data/augment.py` (`LetterBox`),
  `data/loaders.py` (greyscale read)
