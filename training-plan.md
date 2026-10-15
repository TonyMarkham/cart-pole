# Training Plan — YOLO Detector from the Unity Synthetic Pipeline

A detailed, step-by-step plan for using this repo's synthetic data generation
to train the object detector that later runs on the Raspberry Pi 5 (global
shutter camera, ONNX). Frame count is unbounded by design — the constraint is
environment variety and the failure-mining loop, not "infinite data".

## 0. Facts this plan is built on

Verified against landed sources and measured on 2026-10-14. Anything not
measured or source-inspected is marked in Appendix E.

### The pipeline (play mode, editor)

| Component | File | Role |
|---|---|---|
| `RigStateRandomizer` | `Assets/Cart-Pole/Vision/RigStateRandomizer.cs` | Applies M forced situations (list order = capture order, frames 0..M-1) then N random states; randomizes cart slider, pole hinge, desk, skybox (+ yaw 0–360°), light intensity + yaw (front-hemisphere guarantee); fires `StateApplied` per state, `RunCompleted` once. |
| `RigStateCapture` | `Assets/Cart-Pole/Vision/RigStateCapture.cs` | Per state: renders the capture camera, writes frame + YOLO label + manifest + 8K render + 4 marker crops; on `RunCompleted` runs the label verification (editor-only). |
| `ArUcoVerifier` | `Assets/Cart-Pole/Vision/ArUcoVerifier.cs` | Native in-process verify: a label line survives only when its class's retained 8K crop decodes to exactly that class's tag (`Greyhound.Vision.Ghcv.Detect`). Never wipes: zero confirmations refuse the rewrite. |
| `MarkerManager` | `Assets/Cart-Pole/Vision/MarkerManager.cs` | Applies the rig's Marker SOs onto the STEP assembly instance; applies cart/pole states; class id = rig slot order, derived never authored. |
| `RigSO` | `Assets/Rig.asset` | The single binding source: four Marker SOs + cart/pole sibling paths + STEP asset reference. |

### The class-map contract

Class id = the marker's position in the rig's marker array (`Rig.asset`,
verified 2026-10-14 by resolving the array's guids to the SOs in
`Assets/Cart-Pole/markers/`), which matches the corpus `markers.json`:

| class | label | tag id (DICT_4X4_1000) |
|---|---|---|
| 0 | Left | 376 |
| 1 | Right | 596 |
| 2 | Pivot | 920 |
| 3 | End | 937 |

**Requirement — never reorder the rig's marker array.** Every downstream
consumer (labels, `markers.json`, `classes.txt`, dataset yaml, the trained
model's output ids) inherits that order silently.

### Label format

`class x_center y_center width height` — all normalized to [0, 1], origin at
the image's **top left** (y flipped from Unity's bottom-up viewport). One
`.txt` beside each `frame_NNNNNN.jpg`. A marker with any projected corner
behind the camera plane gets no line.

### Measured corpus numbers — the landed corpus is 4 frames
(`frame_000000`–`000003`, 15 boxes), captured docs-off: no `wip~`, no
verification receipt. Rows marked *retired* were measured on the earlier
7-frame corpus, which no longer exists on disk — re-measure from the
first docs-on run.

| Artifact | Size | Notes |
|---|---|---|
| `frame_NNNNNN.jpg` | 97,554 B avg | 1440×1080, JPEG q90 (landed) |
| `frame_NNNNNN.txt` | 142.5 B avg | 15 boxes / 4 frames (landed) |
| `wip~/frame_NNNNNN_8k.jpg` | *retired* 1.13 MB avg | 7680×4320, JPEG q90 |
| `wip~/frame_NNNNNN_8k_marker_TTTT.jpg` | *retired* 8.5 KB avg × 4 | per-marker crops |
| `wip~/frame_NNNNNN_state.json` | *retired* 757 B | per-frame manifest |
| **wip~ total** | *retired* **1.17 MB/frame** | re-measure from a docs-on run |
| **root total** | **~0.098 MB/frame** | the committed artifact (landed) |

Marker box widths in the landed labels: 0.016689–0.023155 normalized →
**24.0–33.3 px** at 1440 px wide (→ 10.7–14.8 px at 640). This drives the
training-resolution decision (Phase 4).

### Current pool inventory

- Desks: 5 `DeskSO`s — `Assets/Cart-Pole/Props/Desks/` (Writing, Standard,
  Panelend, Executive, Classic). The randomizer's `desks` list wires 4 —
  Executive is not yet in it.
- HDRIs: 3 — `Assets/Cart-Pole/Vision/HDRI/` (story_studio_02_4k,
  pav_studio_03_4k, neon_photostudio_4k) → 3 panoramic skybox materials.
- Forced situations: whatever the randomizer's list holds at run time
  (landed: three frame-reproduction SOs, `Assets/Situation-01/02/03.asset`,
  mined from a retired longer run — no camera pose authored).

### Known traps (each one cost time once; do not re-pay it)

1. **The label projection must run with the camera in its render
   configuration.** Landed in `RigStateCapture.Capture` (label written while
   the RT is attached). Measured 2026-10-14: projecting after the target
   restore skews every box ~1.5× too tall in the Game view's aspect, tags off
   up to 128 px. Do not "simplify" the capture order.
2. **Second run into the same directory mixed the corpus** (pre-2026-10-14
   behavior: numbering restarted at `frame_000000`, so a shorter re-run
   left run A's frames stranded beside run B's). Fixed in code, stronger
   than numbering-continuation: a populated target now gets a fresh
   `run_<timestamp>` child directory per play session — runs are
   physically separated, no overwrite, no stranding, no manual step. A
   deliberate append into a populated directory is gone with it.
3. **An untagged capture camera silently disables the front-light
   guarantee.** `FrontYaw()` keys off `Camera.main`; without it the sun yaw
   is uniform ±180°, i.e. back-lit frames become possible.
4. **A STEP reimport breaks the Rig asset's references** (fresh file IDs);
   rebind in the Rig inspector — the stored path string survives for
   fallback, but the live accordions need the rebind.
5. **Replacing any native library the step package ships requires a full
   Editor restart** — Unity maps native plugins once per Editor process.
6. **Never hand-edit a label file to "fix" verification results.** The
   verifier never wipes; a wholesale drop means something systemic broke.
7. `Assets/Cart-Pole/Left.asset` is a blank stray MarkerSO (empty label); the
   rig references `Assets/Cart-Pole/markers/Left.asset`. Don't confuse them
   when checking the rig order.

---

## Phase 0 — One-time scene and tooling checks

Do these once before the first bulk run. Each step has a hand-checkable pass
condition.

1. **Scene** `Assets/Scenes/SampleScene.unity`:
   - Rig's STEP assembly (`asy-cart-pole.step`) instanced in the scene.
   - `MarkerManager`: `rig` = `Assets/Rig.asset`; `robot` = the assembly
     instance (or resolvable); `applyOnStart` = on.
   - Pass: play one state, hierarchy under the assembly shows
     `Marker_0_Left`, `Marker_1_Right`, `Marker_2_Pivot`, `Marker_3_End`.
2. **Capture camera** (`RigStateCapture.captureCamera`):
   - Physical camera, Pi GS sensor gate **4.968 × 3.726 mm**, lens **6 mm**
     (the authored match to the IMX296 rig camera).
   - `captureWidth`/`captureHeight` = **1440 / 1080** (the component's code
     default is 960×540 — the scene must hold 1440×1080; the landed corpus
     proves it does).
   - Pass: the written frames are 1440×1080.
   - **Tag the camera `MainCamera`.** Pass: `Camera.main` resolves to it.
3. **`RobotBounds` cube** in the scene, fitted so the camera's frame
   envelope covers the rig's workspace. The jitter aims at its center and
   frustum-fits against it. The landed scene's capture component leaves
   `robotBounds` unassigned — the `FindAnyObjectByType` fallback covers a
   scene cube, but assign it explicitly. Pass: `RigStateCapture` logs jitter
   offsets per frame, not the "no RobotBounds cube" warning.
4. **Pools**:
   - More desks: drop `.fbx` into `Assets/Cart-Pole/Props/Desks/`, run
     **Cart-Pole → Register Desks** (idempotent; existing DeskSOs untouched;
     new SOs land in `Assets/Cart-Pole/Desks/`), tune each SO's
     position/euler/scale in the inspector, add the SO to the randomizer's
     `desks` list (landed: 4 of 5 wired — Executive missing).
   - More HDRIs: drop `.exr` files into `Assets/Cart-Pole/Vision/HDRI/`, run
     **Cart-Pole → Import HDRIs as Skyboxes** (linear float import, clamp
     wrap, 4k cap, Skybox/Panoramic materials), add the new materials to the
     randomizer's `skyboxes` list.
   - `directionalLight` assigned on the randomizer.
5. **Wiring** (`RigStateCapture` inspector): `randomizer` → scene's
   `RigStateRandomizer`; `captureCamera` assigned; `outputDirectory` = the
   runs' base directory (each play session gets its own subdirectory
   automatically — Phase 2); `jpegQuality` stays **90** — **Requirement:** the corpus and
   the live `StateServer` loop both encode JPEG q90; the trained model must
   see the same lossy statistics at train and inference time.
6. **Sanity run** (one state): **Cart-Pole → Dataset Generator** is NOT part
   of this pipeline — use the randomizer's **Randomize Once** context-menu
   for a visual check. It writes nothing: it is edit-mode, and the capture
   component only subscribes in play mode.

## Phase 1 — Pilot run and the QA gate

1. **Author forced situations** — Create → **Cart-Pole → Forced Situation**.
   Recommended starter set (values are meters/radians as the SO fields state):
   - `cart-left`: cartPosition −0.45 (rail extreme), poleRotation 0.
   - `cart-right`: cartPosition +0.45, poleRotation 0.
   - `pole-extreme-left`: cartPosition 0, poleRotation −`poleSpan` (−3.0 rad).
   - `pole-extreme-right`: cartPosition 0, poleRotation +3.0 rad.
   - `light-dim` / `light-bright`: `useLight` on, intensity 2.0 / 4.0, yaw
     absolute about the light base's x/z. No landed manifest exists (the
     landed corpus was captured docs-off); realistic values until the first
     docs-on run: −23.7, 75.5, 33.8 (`lightYaw` in
     `Assets/Situation-01/02/03.asset`).
   - Optional shot reproduction: `useCameraPosition` on with recorded
     `cameraPosition` + `cameraRotation` from a manifest reproduces a frame
     exactly and skips jitter for that frame.
   Add all of them to the randomizer's `forcedSituations` list. Three
   frame-reproduction situations already exist and are wired
   (`Situation-01/02/03` — cart/pole/desk/skybox/light reproductions from a
   retired run); keep or remove them, but count them: list order = capture
   order, so with all nine the forced frames are 0–8.
2. **Pilot settings.** `RigStateRandomizer` inspector:

   | Field | Value | Why |
   |---|---|---|
   | states | 300 | enough for a QA gate, not a training set |
   | interval | 0.5 | you are watching it |
   | seed | 1001 (distinct nonzero) | reproducible |
   | cartRange | 0.45 | matches the slider limit |
   | poleSpan | 3.0 | the authored parameter |
   | randomizeDesks / Skybox / Light | on | full environment randomization |

   `RigStateCapture` inspector — the jitter and toggle fields live here,
   not on the randomizer (verified in source; the scene's serialized value
   in parentheses):

   | Field | Value | Why |
   |---|---|---|
   | positionJitter | 0.15 (scene: 0.5) | the component's code default |
   | jitterSeed | 2001 (scene: 0), distinct nonzero | reproducible |
   | generateDebugImages / Docs | on (scene: both off) | the verification receipt |

   `seed = 0` means "random each run" (randomizer and jitter alike) — use
   it only when you want fresh noise.
3. **Run**: enter Play mode in the editor (the verify step is editor-only).
   Console gives you, in order: the plan line (`plan: M forced + N random
   states, forced first`), per-frame jitter lines, `done - M forced + N
   random states applied`, then the verification summary line.
4. **QA gate — read `wip~/verification_stats.json`**:
   - `framesScanned` == M + N (else a state's capture was skipped — find out
     why before anything else).
   - `labelsConfirmed` ≈ 4 × frames − drops.
   - `labelsDropped` small and scattered: acceptable — a drop means that
     marker had no crop (behind the camera) and its box was dropped rather
     than kept. The retired 7-frame corpus's receipt read 27 confirmed /
     1 dropped in 28 boxes; the landed 4-frame corpus has no receipt
     (docs-off capture) — the pilot re-earns this precedent.
   - `WriteRefused` true (zero confirmations): **systemic failure** — the
     native detector didn't see anything, e.g. crops missing, class map
     broken, or the ghcv bridge not loaded. Fix the cause; do not scale.
5. **Eyeball before trusting**: **Cart-Pole → Capture Viewer** and
   **Cart-Pole → Run Report** — boxes on markers, crops decode to the right
   tags, desks/skyboxes/light visibly vary.
6. **Record** the pilot's wall-clock (from Play start to the `done` line) and
   the directory size — both feed Phase 2's budget. Nothing in this repo has
   measured throughput yet; the pilot is the measurement.

## Phase 2 — Scale-out campaign

1. **Run separation is code-enforced**: point `outputDirectory` at a base
   (e.g. `Assets/Captures/cart-pole-r002`); an empty base is used flat, a
   populated one automatically gets a fresh `run_<timestamp>` child per
   play session — no manual fresh-directory step exists, and corpora
   cannot mix.
   Naming suggestion: `Assets/Captures/<slug>` where the slug carries the
   config, e.g. `cart-pole-r002-s1002`. The landed
   `Assets/Captures/cart-pole` is a 4-frame docs-off smoke corpus (no
   receipt — Phase 3 refuses it), not the pilot.
2. **Run matrix** — keep it in this file or a sidecar log; one row per run:

   | run | states | seed | jitterSeed | pool delta | notes |
   |---|---|---|---|---|---|
   | r001 (pilot) | 300 | 1001 | 2001 | — | the QA gate |
   | r002 | 1000 | 1002 | 2002 | +2 desks | |
   | r003 | 1000 | 1003 | 2003 | +2 HDRIs | |
   | … | … | … | … | … | new seed + jitterSeed every run |

   `interval` can drop to 0.05–0.1 for bulk runs — it is pure pacing; the
   state application and both renders are synchronous, nothing settles
   between states.
3. **Grow the environment between batches, not just the frame count.** Past
   ~10k frames, more HDRIs/desks buy more real-world transfer than more
   frames (recommendation). The pools are the actual diversity budget:
   3 skyboxes × 5 desks is thin; target 10+ of each before judging transfer.
4. **Throughput budget**: per frame the pipeline renders 1440×1080 **and**
   7680×4320 (21.3× the pixels) plus JPEG encodes — expect the 8K pass to
   dominate. Measure the pilot (Phase 1 step 6) and extrapolate; a 20–50k
   corpus is an overnight/batch affair of many runs either way.
5. **Disk and git math** (estimate from the retired 7-frame corpus's
   per-frame sizes — re-measure from the first docs-on run): 30k frames ≈
   **3 GB** of root corpus into git (JPEG deltas poorly; budget ~3–4 GB
   pack growth) and ≈ **35 GB** of `wip~` sidecars. Two RigStateCapture
   toggles (both default on) govern **artifacts, never verification**:
   `GenerateDebugImages` retains the 8K render + crops as files — with it
   off the 8K render still happens, the crops are still cut, detected in
   memory at capture time, and discarded (saves ~1.14 MB/frame and the 8K
   JPEG encode). `GenerateDebugDocs` writes the run's debug JSON —
   per-frame `_state.json` manifests and `verification_stats.json` — with
   it off the accounting lives only in the Editor log. `markers.json` is
   not debug and not docs-gated: the dataset's class map, written to the
   run root unconditionally and committed beside the labels it
   interprets. The run-end verify always merges the in-memory confirmations
   and rewrites the labels to confirmed lines. Consequence: a docs-off run
   has no file-level verification receipt, so Phase 3's provenance gate
   refuses it — the toggle is for iteration runs; a corpus meant for the
   training set comes from a docs-on run.
6. **`wip~` lifecycle**: keep a run's `wip~` until its
   `verification_stats.json` is accepted; then delete it (recommendation).
   The labels are confirmed/rewritten at that point; any later re-verify on a
   pruned directory finds zero crops → zero confirmations → the never-wipe
   refusal keeps the confirmed labels as written. Deleting `wip~` never
   endangers accepted labels — that is what the refusal is for.
7. **Unity import pressure**: at tens of thousands of frames, Unity imports
   every root `.jpg` (and mints `.meta` files) — AssetDatabase refresh slows
   the editor. Recommendation: after a run is verified and committed,
   `git mv` the run directory out of `Assets/` (e.g. to `datasets/` at the
   repo root) so Unity stops importing it. Tradeoff: the run leaves the
   Unity project tree — nothing in the pipeline needs it there anymore once
   verified.
8. **Commit per run** — one run's root frames+labels per commit keeps the
   corpus reviewable and the history bisectable.

## Phase 3 — Dataset assembly

**Boundary — requirement:** no OpenCV/ArUco anywhere in this phase. The
verification happened in Unity (Phase 1/2 gate); the assembly scripts only
copy, split, and count. All Python via `uv`, never pip; installing anything
needs explicit permission first.

1. **uv project** `training/` at the repo root:
   - `training/pyproject.toml` — assembly needs **stdlib only**
     (`pathlib`, `shutil`, `json`, `random`): zero installs for this phase.
2. **Layout** (hardlink from the run dirs where possible — Windows
   hardlinks avoid duplicating gigabytes; fall back to copy):

   ```
   training/datasets/cart-pole/
     images/train/*.jpg      images/val/*.jpg
     labels/train/*.txt      labels/val/*.txt
     cart-pole.yaml
   ```

3. **`cart-pole.yaml`** — write it from the run's `markers.json` (dataset
   root, committed), in rig slot order:

   ```yaml
   path: D:/git/cart-pole/training/datasets/cart-pole
   train: images/train
   val: images/val
   names:
     0: Left
     1: Right
     2: Pivot
     3: End
   ```

   **Requirement:** the yaml's class order is the rig slot order (Section 0).
   The per-dataset `markers.json` is the record of what each corpus was
   labeled with — keep it beside the run.
4. **Split by run directory, not by frame** (recommendation): frames within
   a run share scene, pools, and jitter seed — a per-run split keeps
   near-duplicates out of val. Ratios ~90/10 at pilot scale, ~95/5 at scale
   (train/val only — the third bucket is played by Phase 5's real-camera
   set). Stratify with the
   `_state.json` manifests' `situation` field: forced situations land in
   train, with one copy of each in val so every hard case is measurable.
5. **Scripts to write** (spec, not landed):
   - `training/scripts/build_dataset.py` — walks run directories, pairs
     `frame_NNNNNN.jpg` ↔ `.txt`, splits by run + situation, hardlinks into
     the layout above, writes the yaml. Refuses to build from a run whose
     `verification_stats.json` is missing or has `WriteRefused` — that is the
     only provenance gate a pruned corpus can offer (its stats json travels
     with the run; keep it when pruning `wip~`, or copy it to the run root).
     A missing stats file reads as unverified and is refused — that covers
     `GenerateDebugDocs=off` runs (no file receipt is written) and runs
     that died before `RunCompleted`.
   - `training/scripts/dataset_stats.py` — per-class box counts, box-size
     histogram (normalized → px at 1440), per-run/situation counts. Pure
     text parsing; no image decoding.
6. **Integrity checks before training** (each is a hand-checkable rule):
   - every `.jpg` has a `.txt` beside it and vice versa;
   - every label line parses to 5 fields, classes ⊆ {0,1,2,3}, coords in
     [0,1];
   - empty label files exist only where all four markers were genuinely
     behind the camera (expect near-zero; investigate any run that produces
     many);
   - spot-check a few frames are 1440×1080;
   - class histogram: expect class 3 (End) underrepresented — the pole swings
     wide (`poleSpan` 3.0 rad) and exits the frame more often. Fix by adding
     runs, never by touching labels.

## Phase 4 — Training

**Permission gate — requirement:** `uv add ultralytics` (pulls torch, ~GB
scale) happens only after explicit OK. Every command below assumes that OK
was given.

1. **Commands** (from `training/`):

   ```sh
   # the base run
   uv run yolo detect train model=yolo11n.pt data=datasets/cart-pole/cart-pole.yaml \
     imgsz=1440 batch=-1 epochs=200 patience=50 device=0 \
     project=runs name=cp-1440-base

   # the deployment-scale comparison (same data, one knob changed)
   uv run yolo detect train model=yolo11n.pt data=datasets/cart-pole/cart-pole.yaml \
     imgsz=640 batch=-1 epochs=200 patience=50 device=0 \
     project=runs name=cp-640-base
   ```

2. **Resolution decision** — the reason `imgsz=1440` is the base: markers
   measure 24.0–33.3 px wide at 1440 (measured from the landed labels). At
   640 they are 10.7–14.8 px — below stride-8 anchor comfort. The two runs
   above
   are the single comparison that decides whether the Pi can afford
   native-scale inference; the 640 run is worth training once, then the
   answer is settled. For the 640 run, also try the small-object variant
   `yolo11n-p2.pt` (adds the P2/stride-4 head) — at 1440 the standard head
   stack is fine.
3. **Hyperparameters** (Ultralytics defaults are mostly right; the ones that
   matter for this domain):
   - `imgsz=1440` — native scale, per above.
   - `fliplr=0.5` (default) — the rig is left/right symmetric-ish; a mirrored
     frame is a valid frame.
   - `degrees=0`, `perspective=0` (defaults) — recommendation: keep them off.
     The jitter already explores position; the physical camera never rolls,
     and perspective warp fakes a lens the rig does not have.
   - `copy_paste` stays 0 — it needs segmentation labels, which this corpus
     does not carry (AABB only).
   - `hsv_*`, `mosaic`, `scale`, `translate` defaults — the environment
     randomization already covers appearance; defaults add robustness on top.
   - JPEG: **train on the frames as written** — no re-encoding, no PNG
     copies. The q90 statistics are part of the domain.
4. **Reading the metrics**: per-class mAP50-95 first (4 classes — aggregate
   mAP hides a failing End marker), then the confusion matrix, then
   precision/recall at high IoU with boxes this small. Pick checkpoints by
   val, not train loss.

## Phase 5 — Export, deployment, sim-to-real

1. **Export**:

   ```sh
   uv run yolo export model=runs/cp-1440-base/weights/best.pt format=onnx imgsz=1440
   ```

   - `half=False` for the Pi's CPU runtime (FP16 is a GPU win; on CPU it can
     be slower or unsupported). Default opset unless the Pi's onnxruntime
     complains.
   - Preprocessing contract: the ONNX input is the letterboxed square —
     inference on the Pi must letterbox the same way Ultralytics does
     (scale to fit, pad). Validate the exported model inside the uv env
     (`onnxruntime` — permission-gated install) against a handful of corpus
     frames before it ever touches the Pi.
2. **The closed loop on this machine** (MuJoCo policy → rendered frames):
   - `StateServer` (`port` 5555) speaks one JSON object per line — either
     `{"id": N, "joints": [{"name": "02-Cart.Slider.joint", "qpos": x},
     {"name": "04-Pole.Hinge.joint", "qpos": a}]}` or the rig payload
     `{"id": N, "rig": {"apply": true, "cartPosition": x, "poleRotation": a}}`
     — and answers with a 4-byte little-endian length header + JPEG bytes.
   - The Python client is `vision/gym_state_sender.py`
     (`UnityVisionClient.send_state`); the future Rust sender (greyhound
     `vision.md` V4 — **plan, not landed**) must speak the same wire format.
   - **Requirement:** `StateServer.captureWidth/Height` → 1440/1080 (its code
     default is 960×540). The model was trained on 1440×1080 q90 JPEGs; the
     loop must render at exactly that.

3. **Pi-side camera discipline** (recommendation): lock AE/AWB on the
   global-shutter camera. The synthetic renders have fixed exposure; the
   real camera's auto-exposure is a domain gap you can eliminate on the
   hardware side for free. Mount the camera to match the authored physical
   camera (gate 4.968×3.726 mm, 6 mm lens ≈ the rig's geometry).
4. **Real-world validation set** — this, not synthetic val mAP, decides
   readiness:
   - collect real 1440×1080 frames from the rig (a few hundred boxes across
     poses/lighting),
   - hand-label them (any labeling tool; installing one is permission-gated),
   - evaluate the exported ONNX against it:
     `uv run yolo detect val model=runs/cp-1440-base/weights/best.pt data=<real-val>.yaml imgsz=1440`.
   - Starting bar (recommendation, adjustable by you): per-class AP50 ≥ 0.90,
     no class below 0.80, on the real set.
5. **Failure-mining loop** — the mechanism the forced-situation design
   exists for:
   - every real-world failure mode (desk occlusion, a specific pole angle,
     a light direction, a cart position) becomes a `ForcedSituationSO`
     reproducing it verbatim — `useCameraPosition` for the shot, `use*`
     flags for desk/skybox/light,
   - once authored, that hard case enters **every** future run automatically
     (forced states run first, frames 0..M−1),
   - regenerate the affected runs, rebuild the dataset, retrain.
   - Synthetic-only hard cases are also worth authoring directly: the pole
     fully horizontal at the rail extremes, a desk overlapping the cart, the
     End marker at maximum swing.

---

The loop to internalize: **runs are cheap and unbounded — the constraint is
environment variety and the failure-mining loop, not frame count.** The
Phase 1/2 verify gate is the only quality control the corpus has; keep it
green before every scale-up.

## Appendix A — Artifact dictionary (what the pipeline writes, per run)

Root (committed; AssetDatabase-imported; the only artifacts git carries):

- `frame_NNNNNN.jpg` — the 1440×1080 camera frame, JPEG q90.
- `frame_NNNNNN.txt` — YOLO label, normalized, top-left origin.
- `markers.json` — the class map (class id → label, tag), written once per
  run to the root, never toggled off: the labels' class ids are
  uninterpretable without it, and Phase 3's yaml is built from it.

`wip~` sidecar (Unity-ignored via trailing tilde, git-ignored via `wip~/`).
Created only when at least one toggle is on — both off leaves the run
directory with nothing but frames, labels and `markers.json`:

- `frame_NNNNNN_8k.jpg` — 7680×4320 render through a camera sharing the
  capture camera's transform and FOV (16:9 — same vertical FOV, wider
  horizontal). Exists for the verify step only. Written only when
  `GenerateDebugImages` is on; off runs detect in memory and retain nothing
  for the 8K pass.
- `frame_NNNNNN_8k_marker_TTTT.jpg` — one crop per marker (tag zero-padded
  to 4 digits), cut from the 8K readback via projected quad corners + 20 px
  margin. Same retention rule as the 8K render. A missing crop means that
  marker was behind the camera — no YOLO data for it in that frame.
- `frame_NNNNNN_state.json` — the manifest: situation label ("" for random
  frames), cart/pole, desk/skybox/light with their use flags, camera pose.
  Written only when `GenerateDebugDocs` is on.
- `markers.json` — moved to the dataset root (see above; the wip~ copy is
  legacy only).
- `classes.txt` — labels in class order (from the `Marker_*` quads).
  Follows the `wip~` directory, not the docs gate: written whenever the
  sidecar exists; a both-off run has neither.
- `verification_stats.json` — the verify accounting, written at run end.
  Same docs gate: a docs-off run's accounting lives in the Editor log only
  (no file-level receipt).

## Appendix B — Verification semantics (why the gate is trustworthy)

- A label line survives only when its class's retained crop decodes to
  exactly that class's tag — one marker per crop, so "nothing", "several",
  or "a different id" all drop the box.
- Single-pass at native crop resolution — the retired Python checker's
  scale-retry is not ported; the validated run was 80/80 crops through the
  native shim.
- Basis decision 2026-10-14: frame-level detection on the 1440×1080 images
  confirmed 66/80 against the crops' 80/80 and was reverted — **this is the
  evidence behind "OpenCV ArUco detection is unreliable at the working
  resolution"**; the 8K render and the crops stay. Detection absence at
  1440×1080 is not a defect signal here; it is exactly why QA runs on
  crops. (The 80-crop corpus predates the landed 4-frame corpus and is not
  on disk — recorded evidence, not reproducible today.)
- The never-wipe rule: zero confirmations on a dataset with labels refuses
  the rewrite and keeps labels as written. Motivating failure: the retired
  Python checker rewrote every label to empty on a 0/80 run. Never
  reintroduce a write path that can empty a label file.
- The in-memory basis (GenerateDebugImages off): the capture detects each
  crop as it cuts it, on the encoded JPEG bytes — byte-identical to what a
  retained file would hold — and hands the run-end verify a stem →
  confirmed-classes map. Retained frames verify from files exactly as
  before; the never-wipe accounting is global across both bases.

## Appendix C — Troubleshooting (known failure signatures)

| Signature | Cause | Fix |
|---|---|---|
| `WriteRefused: true` in stats | systemic: no crops / broken class map / ghcv not loaded | fix the cause before scaling; never hand-edit labels |
| `framesScanned` ≠ M + N | a state's capture was skipped | check randomizer/capture wiring logs |
| boxes skewed ~1.5× too tall | projection after target restore (the trap) | keep the landed capture order; do not reorder |
| sun appears behind the rig | capture camera not tagged `MainCamera` | tag it; `FrontYaw()` needs `Camera.main` |
| no jitter logged | no `RobotBounds` cube in scene | place/fill the cube |
| markers unapplied after STEP reimport | stale file-ID references | rebind in the Rig inspector |
| native detection breaks after package update | Unity mapped the old native lib | **full Editor restart** (native plugins map once per process) |
| mixed corpus | impossible in code | a populated target gets a fresh `run_<timestamp>` child per play session |
| editor log confusion | project-local log stale while the global one is live | check both timestamps; grep the project log for `Failed to open 'Logs/Editor.log'` |

## Appendix E — Status of claims

- **Source-inspected (read, not executed):** every component behavior cited
  in Section 0 and the phases; the Rig array order (guid-resolved);
  StateServer protocol; verify algorithm; the scene's serialized component
  values.
- **Measured (executed commands):** the landed 4-frame corpus's file sizes,
  label contents, box-width range, frame resolution; pool inventory.
  Numbers marked *retired* (wip~ sizes, the 27/1 receipt, the 30k disk
  math) came from the gone 7-frame corpus and are estimates until
  re-measured.
- **Not executed:** any training, any export, any real-rig validation. The
  throughput, mAP, and sim-to-real numbers in this plan are procedures to
  obtain them, not results.
