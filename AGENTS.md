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
Jetson Orin Nano Super (replacing the Raspberry Pi 5:
TensorRT FP16 built from the ONNX export, *The real robot*) with the Pi
global-shutter camera (IMX296) against the real rig. The balance policy
trains in the **synthetic loop** (`synthetic-loop-dia.md`, the source of
truth for the loop plans): the Rust orchestrator steps MuJoCo
in-process, the synthetic Orin triggers the standalone Unity player
(`SimulationLoop.exe`) over WebSocket and detects the markers in its
frames, passing Pivot and End through the matrix its calibration found
(`synthetic-loop-dia.md`, *Calibration*), and the trainer
receives that pool, in metres, for the policy and,
for the reward alone, each capture's true site positions from MuJoCo
and the pole's true speed and centre of mass, with
its mass, inertia and centre-of-mass distance once per run; the policy's input is still the pool. The older
StateServer TCP bridge (`StateServer.cs`) is not part of that loop.

**Not a toy** Every part stands in for the real rig
and keeps its real limits: the MJCF comes from the rig's CAD, the command
is the stepper's pulse rate, and the policy sees only what the robot will
see — detected markers, with the detector's delay — never the
simulator's true state. Gymnasium's own `CartPole-v1` and
`InvertedPendulum-v5` are reference tasks only:
never the task, and never a shortcut around the loop.

**Written for laymen** The repo and all its features
are to be published so that a layman can reproduce and understand every
aspect. Every doc, comment and explanation is written for that reader:
each term explained where it first appears, each number traced to where
it comes from, each step reproducible from the repo alone. No reward
change lands until its ELI-5 explanation is written and the user has
approved it.

**The reward's source of truth is `__synthetic__/trainer/python/reward-plan.md`** 
the ELI-5 explanation of the reward: r = r_tip ×
r_target_speed × r_cart × r_reach × r_speed (r_cart added 2026-10-29;
the same day r_tip's star moved onto the cart and r_reach kept the
fixed one; 2026-10-30: r_target_speed and r_speed replaced
r_energy, plugin `tip_speed_cart_centre`), every input MuJoCo's, its values one `gym.toml` table per factor. The root `reward-plan.md` it replaced was deleted
on 2026-10-28 (in git history at `1e6a1001`); older records that cite it
say so.

**MuJoCo is the source of truth** Wherever MuJoCo
has a value the reward, the episode end or the measures need, the value
is MuJoCo's, sent by the orchestrator: never worked out in Python or Rust
from other numbers, and never a hand calculation standing in for it,
not even as a test's expected value.

The repo owns the **dataset semantics**: desk/skybox/light randomization,
the automated generation sets, capture, YOLO label writing, and the
StateServer bridge. It consumes the **greyhound-snake** packages for
everything below that: STEP import, mass properties, MJCF export, and the
native OpenCV ArUco detection the dataset verification now runs on.

The Unity project's `AGENTS.md`
(`unity/projects/cart-pole/AGENTS.md`) holds the user's standing
directives. They are immutable and bind all work in this repo; this file
adds repo structure around them, never overrides them.

## Working with the user

- **The user's word is the record** "NEVER, EVER TREAT ME
  AS A LIAR AGAIN"). When the user says what they decided or asked for, it is
  written down as they say it now. Never look for proof, never set their
  earlier words against them, never ask them to pick between them. Worked
  failure: told that the no-hit limit had been asked for the day before, the
  agent searched the user's session transcripts for it and asked them to choose
  between two of their own messages.
- **Read nothing on the user's computer outside this repo**.
  This includes Claude Code's own session
  transcripts, the sibling checkouts (`D:\git\greyhound-snake`,
  `D:\git\unor4-wifi-interrupt-bridge`) and any other folder.
  Where a question needs something outside the repo, ask the user. Worked
  failure: the agent searched six session transcripts for the user's earlier
  words, and before that read the bridge fork's code, unasked.
  **The one exception:** `D:\git\unor4-hal` may be read, read-only. Nothing in it is ever
  written.
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
- **Tunable values live in a toml, never in code**
  Any value someone might change between runs — a rate, a period, a
  count, a length, a port, a seed, a path — goes in a toml the code
  reads (the trainer's `settings.toml`, or a file it names, like
  `[trainer] gym`), so variants can sit side by side. Only fixed facts
  copied from a contract stay as named constants in code, each citing
  its source (the frame size, the rig's top rate, the 2 ms grid).
- **An exact direction is applied as given** When the
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
- **Plans are not truth.** The plans here and the design docs in the
  greyhound repo (`vision.md`, `unity-mujoco.md`, `occt-to-unity.md`) state
  intent and research, not current state. Before asserting what the repo
  does today, verify against landed sources. When citing a plan, name it
  as a plan. (Worked example: the removed `ArUco.md` specified
  `DICT_4X4_50` ids 0–3; the landed markers are `DICT_4X4_1000` tags
  376/596/920/937.)
- **`synthetic-loop-dia.md` is the single source of truth for the
  synthetic loop's design** Where a loop plan
  (a participant plan) disagrees with the diagram, the
  diagram is right and the plan is out of date. A change to the loop's
  design lands in the diagram first; the plans follow it. The diagram is
  still a plan: it is the truth for what the loop is meant to be, never
  for what the code does today — verify that against landed sources, as
  above.
- **Truth flows one way: diagram → participant plans → code** 
  A participant's plan is written from the diagram alone,
  never from the code. Code that disagrees with its plan is not a
  conflict to raise: it is work to do, a step in that plan.
- **Separate requirement from recommendation.** State invariants as
  requirements and defaults as choices. A recommendation never hardens
  into an instruction.
- Clearly distinguish source inspection from an executed build or test.
  Do not claim validation that has not actually happened.
- **No JavaScript** Browser code in this repo is Rust
  compiled to WebAssembly, the choice orin-cam made on purpose to avoid all
  JavaScript except the absolutely necessary interop (`trunk`'s generated
  loader and calls through `web-sys`). See
  `__live__/orin/wasm/viewer` and `__live__/orin/wasm/arduino`.
  Never hand-write JavaScript. Worked failure: the first `/arduino` page was
  about 165 lines of it, with the viewer crate right beside it.
- **Clippy must be clean, and `unwrap()` is forbidden except in tests**. The workspace's `Cargo.toml` denies it, and the root
  `clippy.toml` lets tests use it. Check with `cargo clippy --all-targets
  --no-deps` from the repo root, before saying Rust work is done.
- **Python policy.** Always `uv`, never pip; install nothing without
  permission (a standing directive). No Python on the dataset-verify path —
  the Python checker retired; verification runs in-process through the
  `com.greyhound.vision` package's native shim. Remaining Python lives
  in five places, each for a stated reason: `__synthetic__/trainer/python/` (the RL trainer,
  a `uv` project around Gymnasium, the learning tooling),
  `freecad/tools/step-mass.py`
  (FreeCAD's scripting API is Python-only), `__synthetic__/orin/python/`
  (the synthetic Orin) and `__live__/orin/python/` (the real Orin), both `uv`
  projects because detection on the robot is Python: OpenCV +
  TensorRT beat the Rust POC on the Orin (`detector-plan.md` *Results*,
  removed), and `__object_detection__/train/` (the detector training, a `uv`
  project around Ultralytics).
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
  have been the first response, not the fourth.
- **Corrections get weighed before they get answered.** When the user
  corrects you, the default assumption is that the correction is
  right — weigh it genuinely first. The session record is that the
  weighing almost always confirmed the correction. If, and only if,
  real facts conflict, state the conflict ONCE, briefly, as
  information — never as a defense in force, never bundled with new
  content, never as a default stance that the correction was wrong.
  Treating a steering input as a debate opening is the failure mode.
  The correct response to a correction: apply it, confirm it in one
  line, stop.
- **A file with an outstanding issue is frozen.** While any question,
  correction, or unresolved discussion about a file's content is
  open, that file gets NO edits — not even "helpful" ones. Edits made
  against guessed or unconfirmed state are how a doc ends up declaring
  "landed" conclusions the user never reached (observed: marking a
  still-running training run as early-stopped). The freeze lifts when
  the user resolves the issue or explicitly requests the change; a
  "fix this" instruction IS the resolution.
- **Say what you don't understand**. When a request
  is not fully understood, the first thing said is exactly which part,
  plainly ("I understand the nesting, but not how deep it goes or what
  it replaces"). Never cover it with a "no", a claim of understanding,
  or a guess. Not understanding is not a failure; hiding it is, because
  the user then pays for the guesses. Worked failure: asked whether a
  layout example was understood, the agent said yes, listed four
  questions that showed it was not, then declined the user's offer to
  build the skeleton that would have answered them.
- **Comments state the invariant, the hand-checkable rule, or the
  rationale** — never recite what the adjacent code already says.
- Preserve existing user changes. Do not commit, stage, or discard changes
  unless explicitly requested.
- **Never add any byline to any commit message, ever**.
- **Anything about this repo is recorded in this file, never in agent
  memory**: a rule, decision or fact about the repo
  goes here (or into the plan it belongs to), where the user can read
  and correct it.
- **Never use Claude Code's decision box**. No question box and
  no answer buttons at the end of a turn: no `ask` or `actions` in the
  wrap-up, and no `AskUserQuestion`. A question for the user goes in the
  reply's text.
- **One question at a time**. When the user has
  decisions to make, each message carries one: the situation, the options
  and what each leads to. The next comes only after the user has answered.

## How greyhound content is consumed (verify before asserting paths)

- The Unity project references the packages by `file:` entries in
  `unity/projects/cart-pole/Packages/manifest.json`. Those paths resolve
  **outside this repo**: from `Packages/`, `../../../../../greyhound-snake`
  is the sibling checkout `D:\git\greyhound-snake`, whose `dist/package/`
  is what the editor actually loads.
- The Rust side resolves to the same sibling checkout: `orchestrator.toml`'s
  `mujoco_dir` is `../../../../greyhound-snake/dist/mujoco/…` from
  `__synthetic__/orchestrator/rust/`, the file's folder.
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

- **Removed on 2026-11-11** (user: they no longer added value), all in git
  at commit `ecced2cc`; docs that still cite one say it was removed:
  - folders: `aruco/` (`aruco-gen`, superseded by greyhound's
    `aruco-cli`), `bench-station/` (replaced by `arduino-manager`),
    `calibrate/` (the older WiFi firmware with the jump-limit
    experiment), `training/` (replaced by `__object_detection__/train/`),
    `vision/` (`gym_state_sender.py`), `weights/`, and the tracked files of
    `datasets/`;
  - root docs: `ArUco.md`, `GENERATOR.md`, `bench-station-plan.md`,
    `decisions.md`, `detector-plan.md`, `gymnasium-research.md`,
    `hal-flash-memory-support.md`, `hard-coded-in-error.md`,
    `jump-limit-experiment.md`, `jump-limit-results.md`, `mujoco-plan.md`,
    `orchestrator-plan.md`, `real-arduino-plan.md`, `real-open-issues.md`,
    `real-orin-plan.md`, `real-pi-plan.md`, `reward-recommend-fable.md`,
    `reward-research.md`, `rl-rewrite-plan.md`, `robot-plan.md`,
    `synthetic-arduino-plan.md`, `synthetic-orin-plan.md`,
    `trainer-observations.md`, `trainer-plan.md`, `training-plan.md`,
    `unity-player-plan.md`, `zz_uno-plan.md`.
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
- **The Executive desk is excluded by decision**:
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
- `unity/projects/packages/` — the project's own UPM packages, `file:`
  entries in the Unity manifest: `simulation.cart-pole.synthetic-data-gen`
  (the automated dataset generator, the generation sets' runtime, the
  marker and desk SOs) and `simulation.cart-pole.sim-loop`
  (`SimulationLoopController`, the standalone player's renderer for the
  synthetic loop).
- `__synthetic__/` — the synthetic loop's code, moved here on 2026-11-01
  and kept apart from `__live__/`. Each part is a folder by language:
  `orchestrator/rust/`, `orin/python/` and `trainer/python/`. The Unity project
  and its sim-loop package, `__object_detection__/` and the root plans stay
  where they are. `__synthetic__/README.md` has the layout.
- **The Rust workspace** (`Cargo.toml` at the repo root) — the synthetic
  loop's parts under `__synthetic__/orchestrator/rust/crates/`:
  `orchestrator/` (the loop's driver; it began as the Unity leg's stub),
  `physics/` (MuJoCo in-process, linked from `orchestrator.toml`'s
  `mujoco_dir`, the file beside `crates/`; the MJCF and its names come from
  the trainer's `settings.toml` `[mjcf]` on the orchestrator's command line),
  `arduino/` (the loop's synthetic Arduino).
  Roles by the crates' own header comments. The Rust
  detector crate, `object-detection/`, was retired on 2026-10-23:
  a Pi-era POC whose question the removed `detector-plan.md` answered, the
  Python synthetic Orin being faster on the Orin. Its `ObjectDetection.toml`
  moved to `__synthetic__/orin/python/settings.toml`, the Orin's detector configuration.
- `__live__/` — what runs on the real rig, one folder per computer.
  Each part's own README has the commands. **It is kept apart from the repo**: nothing in it runs on the PC. `__live__/orin/rust` and
  `__live__/rpi/rust` are each a workspace with its crates in `crates/`, pushed
  to and built on that computer, so nothing in them points outside them.
  **The Orin's brain is `__live__/orin/rust/crates/brain`** (its README), the
  Rust orchestrator that runs the balance loop on the rig, with its Python
  worker `__live__/orin/python/src/brain_worker.py` for detection and the
  policy's inference; both built 2026-11-07 from `real-loop-dia.md`, and
  first run on the rig 2026-11-09, where the 6M-step policy (run
  `20261108_115227`) balanced the pole. Only
  the firmware (`__live__/arduino/rust/crates/calibration` and
  `__live__/arduino/rust/crates/controller`) and the browser pages
  (`__live__/orin/wasm`) are built on the PC, as root workspace members.
  **`controller` replaces `calibration`**: `controller`,
  begun 2026-11-05, is the production firmware, and once everything is
  implemented in it, `calibration` goes away. Until then `calibration` is kept
  as it is, for reference.
  **The one exception that runs on the PC**: host tests of
  the firmware's pure rules (the ramp, the soft limit, the command unpacking),
  as checks only, in a small `no_std` library crate beside the firmware that
  the firmware itself uses on the board. Nothing that drives the rig runs on
  the PC.
  The firmware crate keeps one type per file, each file named after its type, and its `use` statements put `use crate::…` first, then a
  blank line, then the external crates.
- `__workbench__/` — measurements on the HAL's bench (begun 2026-11-07):
  the R4 WiFi and R3 on this PC's USB (COM3, COM4), wired as unor4-hal's
  `Validation.md` describes. Every timing is taken on one clock, the R3's
  Timer1. `uno_4r/crates/bench-firmware` is R4 firmware copying the
  controller's MOVING loop with D3 marking each sleep; `uno_r3/` holds the
  R3's sketches (`awake_timer`); `pc/crates/bench-host` is the PC program
  that runs the awake test. The WiFi test (gap 7 of `real-timings.md`) has
  its own pair: `uno_4r/crates/wifi-firmware` (GPT320 as a free-running
  48 MHz clock, stamps over USB) and `pc/crates/wifi-host` (the USB clock
  sync and the fit; the PC's clock is the one clock). The apply test (gap 8)
  has `uno_4r/crates/apply-firmware` (the MOVING loop, D3 flipped per
  command), `uno_r3/apply_timer` (INT1 on D3, input capture on STEP) and
  `pc/crates/apply-host`. All six Rust crates are root workspace members,
  not default members. Gap 1 (the camera) is on the Orin:
  `orin/rust/crates/camera-timing` in its own workspace, `orin/rust`,
  pushed to and built on the Orin like `__live__/orin/rust`. It flashes an
  LED wired to an Orin GPIO pin (`orin/led-circuit.svg`) through
  `gpiocdev`, which compiles only on Linux, so on the PC only the rest is
  checked. Gaps 2–6 (the brain's stages) are `orin/rust/crates/stage-timing`
  in the same workspace: a copy of the brain's loop that drives the live
  Python worker unchanged, stamps each stage, and sends its datagrams to the
  PC, not the rig's Arduino. Nothing in `D:\git\unor4-hal` is changed:
  the bench is borrowed, and the R3's validator part is re-flashed from
  there afterwards. `__workbench__/README.md` has the steps.
- `__synthetic__/orin/python/` — the synthetic Orin, a `uv` project: Python + TensorRT
  detection, the 6-entry pool, the WebSocket client.
  The root holds the uv files and `settings.toml` (the detector's
  settings); the code is in `__synthetic__/orin/python/src/`.
- `__object_detection__/` — the detector training's home, begun
  2026-11-10 for the 16,000-frame corpus (`datasets/generated/`, Set-01…16).
  **All of its code is new**: nothing is moved or copied
  from the removed `training/`. Its layout mirrors the old root
  `datasets/` + `training/` pair: `datasets/generated/` (the Unity runs,
  moved here) and `train/` —
  the `uv` project, its `README.md` (every step, with its command and where
  its results go), and `runs/` (gitignored). **`data.yaml` comes from Unity's generator**, as part of the generated data, not from a
  training script.
- **Plans and design docs at the root** — `synthetic-loop-dia.md` (the
  planned sequence of one run, the source of truth for the loop plans);
  `real-loop-dia.md` (the planned sequence of one run on the real rig, the
  single source of truth for the real loop); `rig-drive-research.md` (research, not a plan: whether the UNO R4
  WiFi, the DRV8825 and the stepper can make the top speed, with every
  calculation and source; written before the DM542 replaced the DRV8825
  on 2026-10-26);
  `real-timings.md` (not a plan: every gap in time on the real loop, from
  the camera's exposure to the motor's new rate, each with its clock, its
  method and its measured spread, or TBD; begun 2026-11-07);
  `real-timing-conclusion.md` (suggestions, not a plan: what
  `real-timings.md`'s measurements suggest for the synthetic loop's timing
  draws, and how the draws run in MuJoCo's time; 2026-11-08).
  Plans state intent, not current state (*Working with the user*).

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
  2026-10-23).
- **`training/runs/cp2-1440-base/`** — the **metrology model**, imgsz
  1440, for proposals and audits on the 3090. Best mAP50-95 0.982 at
  epoch 324 (early-stopped at 374).

The roles and per-class figures are in `training/05-POST_TRAINING.md`
(Steps 3–4). `training/` was removed on 2026-11-11: its docs are in git at
commit `ecced2cc`, and its runs, never in git, are no longer on this PC's
disk. The Orin keeps its own copy of the deployment model (`model` and
`engine` in `__live__/orin/python/settings.toml`). Its successor is trained
in `__object_detection__/train/`.

## The real robot (what the synthetic loop stands in for)

- **The robot's computer is a Jetson Orin Nano Super**, replacing the Raspberry Pi 5: TensorRT FP16 detects in
  8–10 ms per frame on it, against 80–125 ms for the best Pi 5 options
  (`detector-plan.md`, *Results*, removed). "The Pi" in older plans names that
  computer's role, the robot side of the loop, not the board. Its code
  is built on the device: the repo is cloned there and compiled (for the Pi; unchanged for the Orin). Requirement: every
  dependency of the robot-side code builds there — aarch64 Linux,
  JetPack 6.2.3 (CUDA 12.6, TensorRT 10.3).
- **Live and synthetic are kept apart**. This replaces
  the older rule that each was one source built two ways, and any plan
  that still says "one source" is out of date. `__live__/` and
  `__synthetic__/` share no code: what both need is copied. Detection on
  the robot runs in Python — on the Orin, OpenCV
  pre-processing + TensorRT ran 11.9–14.5 ms per frame against 18.7–19.1 ms
  for the Rust POC in the same session (`detector-plan.md`, *Results*,
  removed).
- **On the rig, the Orin sends the command packets to the Arduino over
  WiFi.**
- **The orchestrator alone times perception**. In the
  synthetic loop it makes each episode's schedule at the reset, from the
  rig's measured stretches (`__synthetic__/trainer/python/settings.toml`
  `[orchestrator]`, `real-timing-conclusion.md`, 2026-11-08), and nothing
  happens between the schedule's events: no render, no detection, no
  message (`synthetic-loop-dia.md`, *Timing*). The synthetic
  Orin holds no timing of its own, and the firmware holds none to remove:
  its ramp is the motor's limit, unrelated to perception. The orchestrator
  has no real counterpart, so on the rig the Orin runs this timer itself
  (`robot-plan.md`, removed).
- **The Uno is dumb**: its firmware has as few
  decision choices as possible. Any firmware behaviour reduces to a fixed
  rule applied per tick or per command; never propose one that makes
  choices. Example, agreed the same day and not yet in
  `synthetic-loop-dia.md` or code: the step-acceleration ceiling, where
  every tick moves the sent rate towards the last command by at most
  `m × dt` (`m` the jump limit, `jump-limit-results.md`, removed; `dt` the
  tick).
- **The product owner dislikes polling and prefers interrupts**. Code waits for an interrupt to
  tell it something happened, rather than checking again and again.
- **The Arduino has the interrupt bridge**. Its WiFi chip
  runs the user's fork of Arduino's bridge firmware
  (`D:\git\unor4-wifi-interrupt-bridge`, branch `feature/interrupt`; the
  board reported revision 0.6.102 on 2026-10-31, and prints it at each
  start). Push mode sends each UDP datagram to the processor unasked, and
  `unor4-hal`'s `UartIrq` (feature `uart-irq`) wakes it
  (`__live__/arduino/rust/crates/calibration/src/wifi.rs`). Push mode does not
  cover TCP, so such messages are datagrams, not web requests.
- **The policy file is in git and only ever changes whole**. `__live__/orin/python/saved/policy.npz` is the trained actor
  for the Orin, written by the trainer's `export_policy.py` from a run folder
  that git ignores, so it is tracked. A new policy replaces the file
  entirely with a fresh export; it is never edited, patched or merged a part
  at a time. The root `.gitattributes` marks `*.npz` binary, so git never
  diffs or merges two versions.
- **Each diagram participant's real counterpart:**

  | Participant | On the real robot |
  |---|---|
  | Orchestrator | none — it stands in for the physical world |
  | MuJoCo | the physical rig |
  | Unity player | the Pi GS camera (IMX296), its frame reduced to 728×544 greyscale |
  | Synthetic Orin | the Orin: detection, and in production the policy |
  | Synthetic Arduino | the Arduino (an UNO R4 WiFi), the DM542 (replacing the DRV8825, 2026-10-26), the stepper and the belt |
  | Trainer | none — training happens only in the synthetic loop |

- **The camera on the Orin**: the Pi Global Shutter
  Camera (IMX296) is validated on the Orin Nano Super for default colour
  captures.
- **Open issue — the camera's greyscale JPEGs on the Orin**: not tested. To find out: what the camera delivers when
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
  stem.
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
  `StateServer.cs`; its only client, `vision/gym_state_sender.py`, was
  removed. The synthetic loop does not use it: the Unity player speaks
  WebSocket messages instead (`synthetic-loop-dia.md`).
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
