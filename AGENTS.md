# Project purpose

The purpose of this project is to prove that Unity can reliably be used in a
robot training workflow:

- As a synthetic Data generation platform
- As a MuJoCo authoring platform
- As an integrated part of the MuJoCo Gym training workflow

This repository builds the **cart-pole**: a physical rail-mounted cart with a
swinging pole, four ArUco markers attached, designed in FreeCAD, imported into
Unity as a STEP assembly (via the greyhound packages), and rendered there to
generate labeled synthetic-vision datasets. The trained detector (YOLO-class,
ONNX) later runs on a Raspberry Pi 5 with a global-shutter camera against the
real rig; the gym/MuJoCo training loop drives the rendered assembly through a
TCP bridge and receives the rendered frames back.

The repo owns the **dataset semantics**: desk/skybox/light randomization,
forced situations, capture, YOLO label writing, the StateServer bridge, and
the gym-side client. It consumes the **greyhound-snake** packages for
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
- **Separate requirement from recommendation.** State invariants as
  requirements and defaults as choices. A recommendation never hardens
  into an instruction.
- Clearly distinguish source inspection from an executed build or test.
  Do not claim validation that has not actually happened.
- **Python policy.** Always `uv`, never pip; install nothing without
  permission (a standing directive). No Python on the dataset-verify path —
  the Python checker retired; verification runs in-process through the
  `com.greyhound.vision` package's native shim. Remaining Python lives
  where its machine is owned and the tool is interactive: `freecad/tools/
  step-mass.py` (FreeCAD's scripting API is Python-only) and
  `vision/gym_state_sender.py` (the gym-side client).
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
- The Rust side resolves to the same sibling checkout: `physics.toml`'s
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
  under `Assets/Cart-Pole/markers/`.
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
- `unity/projects/cart-pole/Assets/Cart-Pole/Vision/` — the dataset and
  bridge scripts: marker authoring (`MarkerSO`, `RigSO`, `MarkerManager`
  + its menus/editors, `MarkerMaterialSet`), joint driving (`JointDriver`),
  dataset generation (`RigStateRandomizer`, `RigStateCapture`, `RobotBounds`,
  `DeskSO`, `ForcedSituationSO` + editor windows/menus, `HdriSetup`),
  the bridge (`StateServer`, `SnapshotCapture`), and the dataset verify
  step (`ArUcoVerifier` + its entry/frame-result/stats types -
  project-side by decision: dataset semantics live here; the greyhound
  package keeps only the `Ghcv` detection bridge and the OpenCV payload
  provisioner). Inventory verified by file list; roles by the files'
  own headers.
- **The Executive desk is excluded by decision** (2026-10-14):
  `Assets/Cart-Pole/Props/Desks/Desk (Executive).asset` is deliberately not
  wired into the randomizer's `desks` list — the desk would poison the
  dataset. The 4-of-5 wiring is the intended state, not an omission: do not
  re-propose wiring it, and do not "fix" its `label: Classic` field (moot
  while unwired).
- **The capture's debug toggles are off by decision** (2026-10-14):
  `RigStateCapture.generateDebugImages` and `generateDebugDocs` stay off for
  every run, iteration and training alike. Verification is unaffected — the
  crops are cut and detected in memory on the encoded JPEG bytes either way.
  The file-level receipt (`verification_stats.json`) and the per-frame
  manifests are deliberately forgone. Do not re-propose flipping either
  toggle on, including for pilot/training runs, and treat the plans' and
  audit's instructions to the contrary as superseded.
- `unity/projects/cart-pole/Assets/Captures/cart-pole/` — the capture
  corpus: main frames, YOLO labels and `markers.json` (committed); 8K
  renders, per-marker crops, manifests and the remaining sidecar JSONs in
  the `wip~` sidecar (Unity- and git-ignored).
- `vision/gym_state_sender.py` — the gym-side `StateServer` client
  (`UnityVisionClient`): pushes `data.qpos` as JSON lines, receives JPEG
  frames. Retiring to a Rust CLI is planned (greyhound `vision.md` V4) —
  plan, not landed.

## Verified contracts

- **Class ids are rig slot order** (the `MarkerManager`/`RigSO` marker
  entry order, index = YOLO class): 0 = Left (tag 376), 1 = Right (596),
  2 = Pivot (920), 3 = End (937). The mapping lands in `markers.json`
  per dataset, written by `RigStateCapture` from the rig's Marker SOs.
- **YOLO labels**: `class x_center y_center w h`, normalized, top-left
  origin (y flipped from Unity's bottom-up viewport). One `.txt` beside
  each `frame_NNNNNN.jpg`.
- **The never-wipe rule**: the verify step writes only confirmed lines
  and refuses the rewrite when confirmations are zero (a label box
  survives only when its class's retained 8K crop —
  `frame_NNNNNN_8k_marker_TTTT.jpg` — decodes to exactly that class's
  tag). Basis decision 2026-10-14: a frame-level trial on the
  1440x1080 image alone confirmed 66/80 against the crops' 80/80 and
  was reverted — the 8K render and the crops stay. The motivating
  failure: the retired Python checker confirmed 0/80 and rewrote every
  label to empty. Never reintroduce a write path that can empty a
  label file. A missing crop means no YOLO data for that marker in
  that frame: the capture writes no crop when the marker's quad is
  behind the camera, and the verify drops the box rather than keeping
  an unverified one (a wholesale absence - no crops at all - trips the
  never-wipe refusal instead).
- **Forced situations are captures, not modifiers**: a run captures
  forced situations first (list order = capture order, frames 0..M-1),
  then the states - total captures = M + N. A forced situation is
  applied verbatim: cart, pole, camera pose when useCameraPosition,
  and the authored desk, skybox (+ yaw), and light (+ intensity) when
  those use* flags are set; a false flag leaves that element at the
  scene's pristine state, deterministic only because forced states run
  before any random one mutates it. The proof lives in the artifacts:
  forced frames' manifests carry the SO's label in `situation`,
  random frames carry "".
- **The label projection must run with the camera in its render
  configuration** (RigStateCapture projects while the capture RT is
  attached, before the target restore). The capture camera is a
  physical camera (Pi GS sensor gate 4.968x3.726 mm, 6 mm lens), and a
  gate fit makes the vertical projection target-aspect-dependent:
  projecting after the restore skews every label in the Game view's
  aspect (measured 2026-10-14: game view ~2:1 -> boxes ~1.5x too tall,
  tags off up to 128 px at the bottom edge, x correct because the gate
  fit anchors the horizontal half-tangent). The same trap applies to
  any code projecting through a physical camera whose target is
  swapped; the 8K marker camera is immune (its RT is never detached).
- **StateServer protocol**: one JSON object per line in (`id`, `joints[]`
  of `name`/`qpos`, optional rig payload); response = 4-byte
  little-endian length header + JPEG bytes. The server side is
  `StateServer.cs`; the client contract is what `gym_state_sender.py`
  speaks. Any future Rust sender must speak the same wire format.
- **Artifact naming** in a capture directory: `frame_NNNNNN.jpg` /
  `.txt` and `markers.json` at the directory root (the 1440x1080 camera
  frames, the YOLO labels, and the class map that makes their class ids
  interpretable - the only artifacts AssetDatabase imports and git
  commits; `markers.json` is never gated by the capture's debug-image/doc
  toggles); every other artifact lives in the `wip~` sidecar child
  (Unity- and git-ignored): `frame_NNNNNN_8k.jpg` (7680x4320),
  `frame_NNNNNN_8k_marker_TTTT.jpg` (per-marker crops, tag zero-padded
  to 4 digits), `frame_NNNNNN_state.json`,
  `classes.txt`, `verification_stats.json`. Consumers read wip~ first
  and fall back to the root for corpora captured before the layout;
  `markers.json` is read root-first with the wip~ fallback.

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
