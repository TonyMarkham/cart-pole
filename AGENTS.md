# Project purpose

The purpose of this project is to prove that Unity can reliably be used in a
robot training workflow:

- As a synthetic Data generation platform
- As a MuJoCo authoring platform
- As an integrated part of the MuJoCo Gym training workflow

This repository builds the **cart-pole**: a physical rail-mounted cart with a
swinging pole, four ArUco markers attached, designed in FreeCAD, imported into
Unity as a STEP assembly (via the greyhound packages), and rendered there to
generate labeled synthetic-vision datasets. The trained detector
(YOLO11n, `cp2-640-base`; see *The trained detector*) later runs on a
Jetson Orin Nano Super (replacing the Raspberry Pi 5, user 2026-10-23:
TensorRT FP16 built from the ONNX export, *The real robot*) with the Pi
global-shutter camera (IMX296) against the real rig. The balance policy
trains in the **synthetic loop** (`synthetic-loop-dia.md`, the source of
truth for the loop plans): the Rust orchestrator steps MuJoCo
in-process, the synthetic Orin triggers the standalone Unity player
(`SimulationLoop.exe`) over WebSocket and detects the markers in its
frames, and the trainer receives only the marker pool. The older
StateServer TCP bridge (`StateServer.cs` + `vision/gym_state_sender.py`)
is not part of that loop.

**Not a toy** (user, 2026-10-24). Every part stands in for the real rig
and keeps its real limits: the MJCF comes from the rig's CAD, the command
is the stepper's pulse rate, and the policy sees only what the robot will
see — detected markers, with the detector's delay — never the
simulator's true state. Gymnasium's own `CartPole-v1` and
`InvertedPendulum-v5` are reference tasks only (`gymnasium-research.md`):
never the task, and never a shortcut around the loop.

The repo owns the **dataset semantics**: desk/skybox/light randomization,
the automated generation sets, capture, YOLO label writing, the
StateServer bridge, and the gym-side client. It consumes the **greyhound-snake** packages for
everything below that: STEP import, mass properties, MJCF export, and the
native OpenCV ArUco detection the dataset verification now runs on.

The Unity project's `AGENTS.md`
(`unity/projects/cart-pole/AGENTS.md`) holds the user's standing
directives. They are immutable and bind all work in this repo; this file
adds repo structure around them, never overrides them.

## Working with the user

- Follow the scope of the user's explicit request. Do not assume permission
  to perform additional work merely because it would advance the project.
- **Unity validation is the user's.** Compilation, scene behavior,
  inspectors, and `Editor.log` are checked in the editor by the user; the
  agent reads the log on request and fixes code until validation passes.
  An error report alone does not authorize executing the remedy or making
  additional edits beyond the named file.
- **A stated decision is settled.** When the user states intent or a design
  choice, record it as a constraint. Do not re-evaluate it, argue for an
  alternative, or re-open it in later answers. The only acceptable
  objection is one sentence showing a contradiction with an established
  constraint or a verified fact.
- **An exact direction is applied as given** (user, 2026-10-24). When the
  user specifies the edit itself — the text, the file, the place — make
  that edit. If it has a consequence, state it once, plainly, in the
  user's terms ("as placed, the Orin receives `--gym` and exits at
  launch"), and then make the direction work, or ask. Never substitute a
  different design, and never undo the user's edit. Worked failure: asked
  to put `"--gym", "gym.toml"` in `settings.toml`'s `command`, the agent
  added a top-level key instead, then a trainer flag, then removed the
  user's line, three times instead of making the line work; the
  objection behind all three ("the array is the orchestrator's argv")
  was never said clearly.
- **Answer what was asked.** Direct answer first, scoped to the file,
  config, or system the user named. No adjacent tours, no burying the
  answer under "for completeness" coverage of nearby topics.
- **A consequence is named once.** State it attached to the decision it
  follows from. Once the user assigns it a home or acknowledges it, it is
  closed — do not re-raise it later.
- **Do not invent decision points.** If a design replaces a mechanism, the
  capabilities that existed only to serve the replaced design are dropped
  with it. Do not re-introduce them as optional extras or "your call"
  questions.
- **Plans are not truth.** `ArUco.md` here and the design docs in the
  greyhound repo (`vision.md`, `unity-mujoco.md`, `occt-to-unity.md`) state
  intent and research, not current state. Before asserting what the repo
  does today, verify against landed sources. When citing a plan, name it
  as a plan. (Worked example: `ArUco.md` specifies `DICT_4X4_50` ids 0–3;
  the landed markers are `DICT_4X4_1000` tags 376/596/920/937.)
- **`synthetic-loop-dia.md` is the single source of truth for the
  synthetic loop's design** (user, 2026-10-23). Where a loop plan
  (a participant plan) disagrees with the diagram, the
  diagram is right and the plan is out of date. A change to the loop's
  design lands in the diagram first; the plans follow it. The diagram is
  still a plan: it is the truth for what the loop is meant to be, never
  for what the code does today — verify that against landed sources, as
  above.
- **Truth flows one way: diagram → participant plans → code** (user,
  2026-10-23). A participant's plan is written from the diagram alone,
  never from the code. Code that disagrees with its plan is not a
  conflict to raise: it is work to do, a step in that plan.
- **Separate requirement from recommendation.** State invariants as
  requirements and defaults as choices. A recommendation never hardens
  into an instruction.
- Clearly distinguish source inspection from an executed build or test.
  Do not claim validation that has not actually happened.
- **Python policy.** Always `uv`, never pip; install nothing without
  permission (a standing directive). No Python on the dataset-verify path —
  the Python checker retired; verification runs in-process through the
  `com.greyhound.vision` package's native shim. Remaining Python lives
  in five places, each for a stated reason: `trainer/` (the RL trainer,
  a `uv` project around Gymnasium, the learning tooling, user
  2026-10-24, `trainer-plan.md`), `freecad/tools/step-mass.py`
  (FreeCAD's scripting API is Python-only), `vision/gym_state_sender.py`
  (the gym-side client), `orin/` (the synthetic Orin, a `uv` project:
  detection on the robot is Python, user 2026-10-23, because OpenCV +
  TensorRT beat the Rust POC on the Orin, `detector-plan.md` *Results*),
  and `training/` (the detector training, a `uv` project around
  Ultralytics).
- **The user understands before anything lands.** Code, comments, and docs
  are written so the reader can verify claims by hand and never have to
  ask what a value means. A question from the user is a defect report —
  fix the artifact at every copy, not just the conversation.
- **Explanations are purpose-first.** When explaining something you
  built — a script, a pipeline, a tool — the FIRST sentence states the
  problem it solves in the user's terms. Concept in one sentence, then
  the shape (reads X, writes Y), then specifics (paths, fields, edge
  cases). Never mechanics-first: leading with folder paths and field
  counts while the *reason the thing exists* goes unstated is noise,
  and it cost three escalations before the convert_proposals.py
  explanation finally led with "real frames have no geometry, so
  labels can only come from guess-then-correct." That sentence should
  have been the first response, not the fourth. (2026-10-16)
- **Corrections get weighed before they get answered.** When the user
  corrects you, the default assumption is that the correction is
  right — weigh it genuinely first. The session record is that the
  weighing almost always confirmed the correction. If, and only if,
  real facts conflict, state the conflict ONCE, briefly, as
  information — never as a defense in force, never bundled with new
  content, never as a default stance that the correction was wrong.
  Treating a steering input as a debate opening is the failure mode.
  The correct response to a correction: apply it, confirm it in one
  line, stop. (2026-10-16)
- **A file with an outstanding issue is frozen.** While any question,
  correction, or unresolved discussion about a file's content is
  open, that file gets NO edits — not even "helpful" ones. Edits made
  against guessed or unconfirmed state are how a doc ends up declaring
  "landed" conclusions the user never reached (observed: marking a
  still-running training run as early-stopped). The freeze lifts when
  the user resolves the issue or explicitly requests the change; a
  "fix this" instruction IS the resolution. (2026-10-16)
- **Comments state the invariant, the hand-checkable rule, or the
  rationale** — never recite what the adjacent code already says.
- Preserve existing user changes. Do not commit, stage, or discard changes
  unless explicitly requested.

## How greyhound content is consumed (verify before asserting paths)

- The Unity project references the packages by `file:` entries in
  `unity/projects/cart-pole/Packages/manifest.json`. Those paths resolve
  **outside this repo**: from `Packages/`, `../../../../../greyhound-snake`
  is the sibling checkout `D:\git\greyhound-snake`, whose `dist/package/`
  is what the editor actually loads.
- The Rust side resolves to the same sibling checkout: `orchestrator.toml`'s
  `mujoco_dir` is `../greyhound-snake/dist/mujoco/…` from the repo root.
- There is no greyhound submodule (dropped 2026-10-22: nothing built
  from it, and the `dist/` everything loads is gitignored in greyhound).
  The greyhound version anything runs against is whatever the sibling
  checkout has checked out — read its branch and commit before claiming
  a version.
- Work inside the greyhound repo — including its `package/` vs `dist/`
  staging rules — is governed by that repo's own `AGENTS.md`.
- **Restart the Unity Editor completely after replacing any native
  library** the step package ships: Unity maps native plugins once per
  Editor process. (`freecad/EXPORT.md` Troubleshooting records the full
  failure signature.)
- **The Editor log lives in two places; the live one must be found, not
  assumed.** First choice is the project's
  `unity/projects/cart-pole/Logs/Editor.log`; when that open fails the
  session logs that exact line and falls back to the global
  `%LOCALAPPDATA%\Unity\Editor\Editor.log`. Check both timestamps
  (or grep the project-local one for
  `Failed to open 'Logs/Editor.log'`) before reading a log: the
  project-local copy can be stale from an earlier session while the
  global one is live. `Editor-prev.log` beside either is the previous
  session's log.

## Repository map

- `ArUco.md` — the codified marker research: OpenCV 4.x-verified dictionary
  facts, gotchas (bit 1 = white; the black ring is `borderBits`, not the
  codeword), hand-extracted grids, physical sizing, and the camera
  resolution budget (Pi GS camera, IMX296, 1440x1080, ~2.41 m over the
  2000 mm stage). Original plan used `DICT_4X4_50` ids 0–3 — superseded by
  the landed `DICT_4X4_1000` tags; the doc's OpenCV facts still hold.
- `aruco/` — `aruco-gen`, the original standalone DICT_4X4_50 SVG
  generator crate. Superseded by greyhound's `crates/aruco` +
  `aruco-cli` (DICT_4X4_1000, quiet-zone margin, PNG output). Kept for
  reference; do not extend it — greyhound's is the dictionary source.
- `markers/` — the four printed markers as SVG + PNG pairs
  (`marker_0376/0596/0920/0937`), aruco-cli output with the quiet zone
  baked in. These are the physical prints; the Unity-side copies live
  under `Assets/Simulation/Markers/`: `svg/` and `Textures/` (the same
  SVG and PNG per tag), `Materials/` and `prefabs/` (one per tag), and
  the four `MarkerSO` assets `Left`, `Right`, `Pivot`, `End` (verified
  by file list and script GUID, 2026-10-23).
- `freecad/` — the mechanical source of truth: `cad/` (FCStd parts and
  assemblies), `step/` (exported STEP, currently `asy-cart-pole.step`),
  `tools/step-mass.py` (the mass/density patcher — FreeCAD exports no
  mass; the patcher injects AP214 mass + OCCT-canonical material/density
  into the STEP text), and `EXPORT.md` (the procedure and its rules).
  `EXPORT.md`'s rules are binding: never hand-edit a patched STEP, always
  patch after a fresh export, keep STEP and FCStd stems identical, never
  patch while the Editor is running.
- `unity/projects/cart-pole/` — the Unity 6 project (6000.7.0b4, URP).
  Assets compile as Assembly-CSharp/-Editor (no asmdefs in Assets);
  namespace `CartPole.Vision`. One type per file, `.meta` files are
  Unity-minted — never hand-author one, never move or rename tracked
  assets outside the editor without moving the `.meta` with them.
- `unity/projects/cart-pole/Assets/Cart-Pole/Vision/` — the project-side
  dataset and bridge scripts: joint driving (`JointDriver`), dataset
  generation (`RigStateRandomizer`, `RigStateCapture`,
  `ForcedSituationSO`), the bridge (`StateServer`, `SnapshotCapture`),
  and the dataset verify step (`ArUcoVerifier` + its
  entry/frame-result/stats types - project-side by decision: dataset
  semantics live here; the greyhound package keeps only the `Ghcv`
  detection bridge and the OpenCV payload provisioner). `Editor/` holds
  `DatasetGeneratorWindow`, `CaptureViewer`, `CaptureReportWindow`,
  `ForcedSituationEditor`, `MarkerMenu`, `DeskMenu`, `HdriSetup` and
  `StaleSelectionCleaner`. Marker authoring (`MarkerSO`, `RigSO`,
  `MarkerManager`), `RobotBounds` and `DeskSO` have moved into the
  `simulation.cart-pole.synthetic-data-gen` package's `Runtime/`;
  `MarkerMaterialSet` no longer exists. Inventory verified by file list
  (2026-10-23); roles by the files' own headers.
- **The Executive desk is excluded by decision** (2026-10-14):
  `Desk (Executive).asset` (in the `synthetic-data-gen` package,
  `Runtime/Props/Desks/`) is deliberately not wired into any generation
  set's `m_Desks` list (`Assets/Simulation/GenerationSets/Set-*.asset`) —
  the desk would poison the dataset. All 13 sets list the other four
  (Classic, Panelend, Standard, Writing) and none lists Executive
  (verified by GUID, 2026-10-23). The 4-of-5 wiring is the intended
  state, not an omission: do not re-propose wiring it, and do not "fix"
  its `label: Classic` field (moot while unwired).
- **Dataset generation is automated** in the `synthetic-data-gen`
  package: `SyntheticDataGenerator` (wired in `SampleScene`) runs the
  generation sets through `GenerationSetRunner`, `LabelProjector`,
  `VerificationGate` and `DatasetWriter`. It replaced the
  `RigStateRandomizer` + `RigStateCapture` pair (its own header), and
  `RigStateCapture` is referenced by no scene, prefab or asset (verified
  by GUID, 2026-10-23). The old capture's debug-toggle decision
  (2026-10-14: `generateDebugImages` and `generateDebugDocs` stay off)
  applies only to that unwired component; the automated generator has no
  such toggles, and its verification runs in memory and writes no
  instrument files.
- `datasets/` — the dataset trees, in the one format `DATASETS.md`
  defines (Ultralytics' layout: `images/` and `labels/` paired by stem,
  a `data.yaml` at the tree root): `generated/` (the synthetic corpus,
  one `run_Set-NN_<timestamp>/` folder per generation-set run),
  `real-1440/` and `real-640/` (real-rig frames), and `calibration/`.
  Only the real and calibration trees' `data.yaml` files and the
  `.gitignore`s are committed; every image and label is ignored, and the
  generated corpus is regenerable by design (its `.gitignore` says so).
  Verified by `git ls-files datasets` (2026-10-23).
- `vision/gym_state_sender.py` — the gym-side `StateServer` client
  (`UnityVisionClient`): pushes `data.qpos` as JSON lines, receives JPEG
  frames. Retiring to a Rust CLI is planned (greyhound `vision.md` V4) —
  plan, not landed.
- `unity/projects/packages/` — the project's own UPM packages, `file:`
  entries in the Unity manifest: `simulation.cart-pole.synthetic-data-gen`
  (the automated dataset generator, the generation sets' runtime, the
  marker and desk SOs) and `simulation.cart-pole.sim-loop`
  (`SimulationLoopController`, the standalone player's renderer for the
  synthetic loop, `unity-player-plan.md`).
- **The Rust workspace** (`Cargo.toml` at the repo root; `aruco/`
  excluded) — the synthetic loop's parts, each with its plan:
  `orchestrator/` (the loop's driver; it began as the Unity leg's stub),
  `physics/` (MuJoCo in-process, linked from `orchestrator.toml`'s
  `mujoco_dir`; the MJCF and its names come from the trainer's
  `settings.toml` `[mjcf]` on the orchestrator's command line,
  `mujoco-plan.md`), `arduino/` (the Arduino firmware, one source for
  the rig and the loop's synthetic Arduino, `synthetic-arduino-plan.md`).
  Roles by the crates' own header comments and the plans. The Rust
  detector crate, `object-detection/`, was retired on 2026-10-23 (user):
  a Pi-era POC whose question `detector-plan.md` answered, the Python
  synthetic Orin being faster on the Orin. Its `ObjectDetection.toml`
  moved to `orin/settings.toml`, the Orin's detector configuration.
- `orin/` — the synthetic Orin, a `uv` project: Python + TensorRT
  detection, the 6-entry pool, the WebSocket client (`synthetic-orin-plan.md`).
  The root holds the uv files and `settings.toml` (the detector's
  settings); the code is in `orin/src/`.
- `training/` — the detector training, a `uv` project: the numbered
  docs `01`–`07` and appendices, `src/` (`build_dataset.py`,
  `convert_proposals.py`, `dataset_stats.py`), and `runs/` (gitignored;
  see *The trained detector*).
- `weights/yolo26n.pt` — tracked (commit `9297d40a`, "Train object
  detection"); no doc names its use.
- **Plans and design docs at the root** — `synthetic-loop-dia.md` (the
  planned sequence of one run, the source of truth for the loop plans),
  one plan per diagram participant, named after it: `orchestrator-plan.md`, `mujoco-plan.md`,
  `trainer-plan.md`, `synthetic-arduino-plan.md`,
  `synthetic-orin-plan.md`, `unity-player-plan.md`; `detector-plan.md`
  (the retired Rust detector crate's plan, kept for its *Results*: the
  measurements that chose the Orin and Python); `DATASETS.md` (the one dataset format) and
  `GENERATOR.md` (the generator's binding directives, which win over
  `DATASETS.md` where they conflict); `training-plan.md` (the YOLO
  training plan); `gymnasium-research.md` (research, not a plan: what
  Gymnasium 1.4 and Stable-Baselines3 2.9 require of an environment,
  read against the loop, for the trainer's open decisions). Plans state
  intent, not current state (*Working with the user*).

## The trained detector

The YOLO detector is trained (2026-10-17): two YOLO11n runs on
`datasets/generated/data.yaml` (up to 500 epochs, patience 50), in
`training/runs/` (gitignored), each keeping its val-selected checkpoint
at `weights/best.pt`. Best mAP50-95 read from each run's `results.csv`
(2026-10-23):

- **`training/runs/cp2-640-base/`** — the **deployment model**, imgsz
  640, the one the robot runs. Best mAP50-95 0.971 at epoch 494 of 500.
  Beside `best.pt`: `best.onnx` (exported 2026-10-17) and
  `best_fp16.engine` (TensorRT FP16 for this PC's 3090, built
  2026-10-23, `synthetic-orin-plan.md`).
- **`training/runs/cp2-1440-base/`** — the **metrology model**, imgsz
  1440, for proposals and audits on the 3090. Best mAP50-95 0.982 at
  epoch 324 (early-stopped at 374).

The roles and per-class figures are in `training/05-POST_TRAINING.md`
(Steps 3–4).

## The real robot (what the synthetic loop stands in for)

- **The robot's computer is a Jetson Orin Nano Super** (user,
  2026-10-23), replacing the Raspberry Pi 5: TensorRT FP16 detects in
  8–10 ms per frame on it, against 80–125 ms for the best Pi 5 options
  (`detector-plan.md`, *Results*). "The Pi" in older plans names that
  computer's role, the robot side of the loop, not the board. Its code
  is built on the device: the repo is cloned there and compiled (user,
  2026-10-22, for the Pi; unchanged for the Orin). Requirement: every
  dependency of the robot-side code builds there — aarch64 Linux,
  JetPack 6.2.3 (CUDA 12.6, TensorRT 10.3).
- **Live and virtual are one source.** The robot computer's code and
  the Arduino firmware are each one source built two ways: the live
  build for the hardware, the synthetic build for the loop; only the
  edges (where frames come from, where commands go) differ. Exception:
  detection on the robot runs in Python (user, 2026-10-23) — on the
  Orin, OpenCV pre-processing + TensorRT ran 11.9–14.5 ms per frame
  against 18.7–19.1 ms for the Rust POC in the same session
  (`detector-plan.md`, *Results*).
- **On the rig, the Orin sends the command packets to the Arduino over
  WiFi.**
- **Each diagram participant's real counterpart:**

  | Participant | On the real robot |
  |---|---|
  | Orchestrator | none — it stands in for the physical world |
  | MuJoCo | the physical rig |
  | Unity player | the Pi GS camera (IMX296), its frame reduced to 728×544 greyscale |
  | Synthetic Orin | the Orin: detection, and in production the policy |
  | Synthetic Arduino | the Arduino, the DRV8825, the stepper and the belt |
  | Trainer | none — training happens only in the synthetic loop |

- **The camera on the Orin** (user, 2026-10-23): the Pi Global Shutter
  Camera (IMX296) is validated on the Orin Nano Super for default colour
  captures.
- **Open issue — the camera's greyscale JPEGs on the Orin** (user,
  2026-10-23): not tested. To find out: what the camera delivers when
  asked for greyscale (a true single-channel mono/Y format, or colour
  converted after capture), and the JPEGs that result — channel count,
  size, quality. Why it matters: the detector trained on, and the loop
  feeds it, 728×544 q90 frames of BT.601 luma written into three equal
  RGB channels (`synthetic-loop-dia.md`, *Contracts*, the frame). The
  Orin decodes to one channel either way, but the real frames must match
  that image domain.

## Verified contracts

- **Class ids are rig slot order** (the `MarkerManager`/`RigSO` marker
  entry order, index = YOLO class): 0 = Left (tag 376), 1 = Right (596),
  2 = Pivot (920), 3 = End (937). The mapping lands in `markers.json`,
  written once at each run's root by `DatasetWriter` from the rig's
  marker order.
- **YOLO labels**: `class x_center y_center w h`, normalized, top-left
  origin (y flipped from Unity's bottom-up viewport). One
  `labels/frame_NNNNNN.txt` per `images/frame_NNNNNN.jpg`, paired by
  stem (`DATASETS.md`).
- **The never-wipe rule, write-once form** (`VerificationGate`,
  `DatasetWriter`): per captured frame an 8K greyscale render
  (7680x4320) through a telephoto twin of the capture camera is cut into
  per-marker crops in memory, and a label box survives only when its
  class's crop decodes to exactly that class's tag. Each label file is
  written once, with only the confirmed lines, so no rewrite exists; an
  empty label file is legitimate - the gate confirmed nothing, a
  background frame. No crop file is ever written. Basis decision
  2026-10-14: a frame-level trial on the 1440x1080 image alone confirmed
  66/80 against the crops' 80/80 and was reverted — the 8K render and
  the crops stay. The motivating failure: the retired Python checker
  confirmed 0/80 and rewrote every label to empty; the write-once form
  has no rewrite path at all.
- **Forced situations** (`ForcedSituationSO`) belong to the old
  `RigStateCapture` pipeline; the automated generator has none.
- **The label projection must run with the camera in its render
  configuration** (`LabelProjector`'s rule: project while the camera is
  in the configuration that rendered the image). The capture camera is a
  physical camera (Pi GS sensor gate 4.968x3.726 mm, 6 mm lens), and a
  gate fit makes the vertical projection target-aspect-dependent:
  projecting after the restore skews every label in the Game view's
  aspect (measured 2026-10-14: game view ~2:1 -> boxes ~1.5x too tall,
  tags off up to 128 px at the bottom edge, x correct because the gate
  fit anchors the horizontal half-tangent). The same trap applies to
  any code projecting through a physical camera whose target is
  swapped.
- **StateServer protocol**: one JSON object per line in (`id`, `joints[]`
  of `name`/`qpos`, optional rig payload); response = 4-byte
  little-endian length header + JPEG bytes. The server side is
  `StateServer.cs`; the client contract is what `gym_state_sender.py`
  speaks. The synthetic loop does not use it: the Unity player speaks
  WebSocket messages instead (`unity-player-plan.md`, *Settled
  decisions*; `synthetic-loop-dia.md`).
- **Artifact naming** in a generated run (`DatasetWriter`): one
  `run_<set>_<timestamp>/` folder per set run under the set's output
  directory, holding `images/frame_NNNNNN.jpg`,
  `labels/frame_NNNNNN.txt` and `markers.json` at its root (checked
  against `datasets/generated/run_Set-01_20261022_193157/`). There is no
  `wip~` sidecar and no 8K, crop, state or stats file: the instruments
  are in-memory steps. Corpora captured by the old `RigStateCapture`
  used a root + `wip~` layout.

## Code conventions (C#, this repo)

- One type per file, `sealed` where possible, block-scoped namespaces
  (`namespace CartPole.Vision`), Allman braces, 4-space indent, `var`
  by default, comments wrapped at ~80 columns stating rules not
  mechanics.
- File I/O via `System.IO` with interpolated `$"{outputDirectory}/..."`
  paths; JSON via `JsonUtility` with small `[Serializable]` mirror
  classes; no external dependencies beyond the greyhound packages.
- Editor-only code paths stay behind `#if UNITY_EDITOR` inside runtime
  components (see `RigStateCapture.OnRunCompleted`); the component must
  compile for players regardless.
- **Unity 6000.7's lifecycle analyzer (UAL0010/UAL0013) demands an
  annotation on every static type holding static fields, auto-
  properties, or events**: `[AutoStaticsCleanup]` or
  `[NoAutoStaticsCleanup]` from `Unity.Scripting.LifecycleManagement`,
  plus the `partial` modifier on the declaration — the attribute is
  implemented by a source generator that emits the cleanup code into a
  generated partial part, so a non-partial declaration cannot carry
  it. `[NoAutoStaticsCleanup]`
  is the precedent for process-lifetime state — the provisioners'
  payload state, the ghcv load flags — because the state describes the
  process's native module table, which Unity never unloads on scene
  unload or domain reload. Do not "fix" the warning by deleting the
  state; annotate it and say why in the type's doc comment.
