# Unity Player Plan — the loop's renderer

The loop needs a renderer that turns MuJoCo's joint positions into
exactly the frame the Pi's detector will see — a 728×544 greyscale JPEG
at q90 — running as a standalone program the orchestrator launches and
talks to. This plan builds it as the new UPM package
`simulation.cart-pole.sim-loop` plus a Windows player build, and a small
Rust orchestrator stub that launches the player, sends it joint states
over a WebSocket, and checks the frames that come back.

Status (end of 2026-10-22): **the Unity leg works end to end in a built
player** — Done-when 1–4 pass (*Results* below); 5 (editor Play mode with
`--no-launch`) has not been run. Steps 1–7 and 9 are done, step 8's build
half is done. Nothing is committed (*Next*). Unity compilation, scene
behavior, and the build are validated by the user in the editor.

Since (2026-10-24, committed): the player is the Orin's camera
(`synthetic-loop-dia.md` 6–7, 12–13). It takes `-orin ws://…`, the
synthetic Orin's frame socket, and in the editor `m_EditorOrinUrl`
(default `ws://127.0.0.1:9001`, a new field so the old port 9000 is not
carried over). The user compiled it and rebuilt `SimulationLoop.exe`
(2026-10-23); a trainer run with it registers with the Orin, renders 60
frames and exits 0 when the Orin closes the socket. Editor Play mode with
`--no-launch` has still not been run.

**Reset in place** (planned, the diagram's *Reset in place*; user,
2026-10-24): one run holds many episodes, so the player is launched once
per run and its window stays open between episodes, where it can be
placed and watched. It takes no part in a reset: it stays connected and
renders whatever pose comes next, the start pose included. Nothing in the
player changes for it; what is to check is that it stays connected across
a reset. **Done (2026-10-24):** in the orchestrator's reset check and a
short training run (two resets), one player served every episode and
exited 0 at the end.

**The planned loop is `synthetic-loop-dia.md`** (2026-10-23), the
source of truth: it supersedes the decisions marked *Superseded* below.
What this plan built and measured (*Results*) stays as the record of the
leg as it runs today.

## Done when

1. The stub launches the built player; the player connects to the
   stub's WebSocket and logs the address it connected to.
2. The stub receives one frame per joint state it sends; every frame
   is a 728×544 JPEG (read from the JPEG header) and looks grey (no
   colour).
3. The four markers are visible in the frames.
4. A frame rendered at cart 0.2 m / pole 0.5 rad shows the cart on the
   same side and the pole tilted the same way as MuJoCo `simulate` at
   the same joint values (step 9).
5. With `--no-launch`, editor Play mode connects to the stub and the
   same checks pass — no player build needed to iterate.

## Results (2026-10-22)

All runs from the repo root against `Build/SimulationLoop.exe`.

- **Sweep — Done-when 1, 2, 3 pass.**
  `cargo run -p orchestrator -- --player unity/projects/cart-pole/Build/SimulationLoop.exe`:
  the player connected, answered all 60 states on **one connection**
  with 60 JPEGs, each `728x544`, 3 channels, ~42 KB, and quit when the
  stub closed the socket (`player exited: exit code: 0`). This proves
  the persistent lockstep session: the player stays open across many
  requests and closes only when the orchestrator does. `Player.log`:
  `[RigSO] 4/4 markers applied`, `[SimulationLoopController] connected
  to ws://127.0.0.1:9000`, `... orchestrator connection ended`; no
  errors, warnings, or exceptions.
- **Frames inspected** (0, 15, 30, 45 of the sweep): greyscale, all four
  markers visible (both rail ends, pivot, tip). Frame 0 (pole 0): pole
  hangs down. Frame 30 (pole π): pole upright. Frames 15 / 45 (cart
  ±0.4 m, pole 90° / 270°): cart at the right / left end, pole
  horizontal pointing right / left. The pole rides the cart in every
  frame (parent-relative poses).
- **Convention check — Done-when 4 passes.** `--pose 0.2,0.5` rendered
  the cart right of center, about halfway to the right end, with the
  pole tip down and to the right, ~29° from vertical. MuJoCo
  `simulate` paused at `02-Cart.Slider.joint` 0.207 /
  `04-Pole.Hinge.joint` 0.534 shows the same: cart right of center,
  pole tip toward the cart's displacement, ~30°. This confirms
  `MjcfToUnityHingeSign = -1` against MuJoCo itself.
- **Not measured:** speed. The runs looked fast; no timing exists yet.

## Next

1. **Done-when 5:** `cargo run -p orchestrator -- --no-launch`, then
   editor Play mode — the editor half of step 8.
2. **Detector on loop frames** — the next real test of the vision side.
   Run the trained `training/runs/cp2-640-base/weights/best.onnx` on
   `target/orchestrator-frames/`. It was trained on 1456×1088 generator
   frames from randomized camera poses and environments; the loop
   renders 728×544 from one fixed nominal camera. Pass: all four classes
   found where the markers are.
   **Run 2026-10-22 — fails for End when the pole hangs.** Ultralytics'
   own predictor (`imgsz` 640, `conf` 0.25, the training env via
   `uv run --no-sync`), best box per class, with both weights of the
   only 640 run, `cp2-640-base` (`best.pt` 2026-10-17 20:02, and
   `best.onnx` exported from it at 20:19), on every loop frame set: the
   60-frame sweep (`target/physics-sweep/`), the 250-frame position-model
   sine run (`target/physics-render/`), and the 250-frame velocity sine
   run (`target/physics-render-vel/`). The two weights agree except at
   the edge case noted below, so the ONNX export is not the cause.
   - Left, Right, Pivot: pass. Found in every frame where they are
     unobscured; where the pole obscures a rail-end marker (sweep frames
     11–12 Right, 48–49 Left) it is not registered, which is the
     intended behavior — an obscured marker is deliberately not
     registered (user, 2026-10-22). Lowest confidence when found 0.908;
     no frame has more than four boxes. Boxes sit on the markers in the
     frames inspected (sweep 4, 12, 30).
   - **End: missed whenever the pole is within 42° of hanging** — a
     true detector failure: the marker is clearly visible in the missed
     frames (sweep frame 4). Sweep frames 0–7 and 53–59 (pole 0–42° and
     318–354°), and all 250 frames of both sine runs (pole within
     ±0.27 rad). From 48° up it is found (confidence ≥ 0.937). The one
     difference between the weights: `best.pt` finds End in sweep frame 7
     (42°) at confidence 0.660, y = 0.857 — right at the lowest End
     position in the training labels.
   - **Cause: camera framing, not orientation.** The training labels
     contain hanging poses (End below Pivot in 4008 of 9000 frames), but
     no End box lower than y = 0.865 of the frame height (0.9–1.0: none;
     0.8–0.9: 9%). In the loop's view (camera at (0, 0.25, −2.41), level)
     the hanging tip sits below that: the last detection is at y = 0.833
     (48°), the misses begin where the tip passes ≈ 0.86. The model has
     never seen an End marker that low in a frame.
   - **Background ruled out (2026-10-22).** The loop scene's skybox was
     switched from `neon_photostudio_4k` to `story_studio_02_4k` (one of
     the generator's own skyboxes) and the player rebuilt; the sweep and
     velocity sine run were re-rendered (`target/story-sweep/`,
     `target/story-render-vel/`). Same result with both weights: End
     missed in sweep frames 0–7 and 53–59 (`best.pt` also 52) and in all
     250 sine frames; Left, Right, Pivot as before. The detections at
     the edge got weaker, not stronger: End at y = 0.833 (48° / 312°)
     dropped to confidence 0.829 / 0.470 (`best.onnx`) and 0.281 / missed
     (`best.pt`), from ≥ 0.937 on neon.
   - **Mount pose — passes (2026-10-22).** The user moved the loop
     scene toward the measured real setup (floor to pivot 1080 mm, floor
     to camera 990 mm, camera to the Pivot marker 2000 mm): rig and
     `desk_2_sitstand` at y = 1.08, camera at (0, 0.99, −2.41), level;
     player rebuilt. Re-rendered sweep (`target/mount-sweep/`) and
     velocity sine run (`target/mount-render-vel/`), both weights:
     **End found in every frame** (lowest confidence 0.938); all four
     found in all 250 sine frames and 55 of 60 sweep frames. The five
     sweep misses are rail-end markers the pole lies across (frames
     10–12 Right, 49–50 Left), not registered by intent. Detected
     positions match the lens-model prediction for this pose: frame 0
     Pivot y 0.3781 (predicted 0.378), rail y 0.4207 (0.421), hanging End
     y 0.6976 (0.696). The camera now sits below the rig, so the hanging
     tip lands at 0.70 — inside the trained range. The scene is not yet
     exactly the measured pose: the Pivot marker sits 0.082 m above and
     0.132 m in front of the rig root, so the camera is 172 mm below and
     2278 mm in front of it (measured: 90 mm, 2000 mm).
   - **Mount pose, heights matched — passes (2026-10-22).** Rig base
     and desk top at y = 1.000 (the Pivot marker at 1.082), camera at
     (0, 0.99, −2.41): 92 mm below the Pivot marker (measured 90 mm),
     still 2278 mm in front of it (measured 2000 mm). Re-rendered
     (`target/mount2-sweep/`, `target/mount2-render-vel/`), both
     weights: End found in every frame (lowest confidence 0.928); all
     four in all 250 sine frames and 56 of 60 sweep frames, the misses
     again rail-end markers the pole lies across (frames 10–12 Right, 49
     Left). Frame 0 positions match the prediction: Pivot y 0.4349
     (predicted 0.435), rail y 0.4759 (0.476), hanging End y 0.7544
     (0.753) — and the lowest End in the 146 real frames is 0.756.
   - Not decided: whether the fix is the loop's camera pose (itself a
     placeholder for the real mount, *Placeholders*) or the generator's
     camera ranges and a retrain. The check script and per-frame CSVs
     are scratch files, not committed.
3. **Timing:** per-frame round trip and sustained frames per second in
   the stub, to compare with the Pi's frame interval T (the retired `loop-plan.md`).
   Proposed as a separate session ("Add frame timing to the orchestrator
   stub").
4. **Step 3's remaining check:** one dataset-generator run, frames as
   before.
5. **Commit** (nothing is committed). cart-pole: `.gitignore`,
   `Cargo.toml`, `Cargo.lock`, `orchestrator/`, `loop-plan.md`,
   `unity-player-plan.md`, `unity/projects/packages/simulation.cart-pole.sim-loop/`,
   synthetic-data-gen's `GreyscaleEncoder.cs` (+ `.meta`) and
   `GenerationSetRunner.cs`, `Packages/manifest.json` and
   `packages-lock.json`, `Assets/SimulationLoop/` (+ `.meta`), the
   `Windows (Loop)` build profile (+ `.meta`),
   `EditorBuildSettings.asset`, `01-Cart-Pole.root.asset`.
   greyhound-snake: `mujoco-win.md`.
6. **Then the other legs** of the retired `loop-plan.md` — physics worker, virtual
   Pi, firmware, and the real orchestrator — each with its own plan.

## Facts this plan builds on

- **`StateServer.cs`** (`Assets/Cart-Pole/Vision/`, project-level, no
  asmdef) — the reference for the render path, **not pulled in**: TCP
  on `127.0.0.1`, `port` default 5555; one JSON object per line in —
  `id`, `joints[]` of `name`/`qpos`, optional `rig` payload applied
  through `MarkerManager.ApplyState`; reply = 4-byte little-endian
  length + JPEG. It renders an assigned camera into a
  `RenderTexture(captureWidth, captureHeight, 24)`, reads back `RGB24`,
  encodes `EncodeToJPG(90)` — **colour, 960×540** (field defaults).
  Requests arrive on background threads and are handed to `Update`
  through a queue, because Unity APIs are main-thread only.
- **`JointDriver.cs`** (same folder): maps MJCF joint names to
  `StepJoint` assets (Greyhound.Step) and drives them — slide
  translates along the joint axis by `qpos` meters, hinge rotates about
  the axis through the joint pivot by **`+qpos`** radians — using the
  joint asset's `mj.axis`/`mj.pos`. Base poses are captured at
  `qpos = 0`. Its configuration is scene-bound: `assemblyRoot` is a
  scene reference to the STEP instance, and each entry's `part` is a
  `Transform` it writes into a serialized field.
- **Neither script is referenced** by any scene, prefab, or asset
  (GUID search of `Assets/`).
- **`StepJoint` / `StepJointSet`** (Greyhound.Step, ScriptableObjects):
  a `StepJoint` holds its part's sibling-index path (`body`, a
  `StepSiblingPath`) and its MJCF mirror (`mj`: `type`, `pos`, `axis`,
  limits). The MJCF exporter names each `<joint>` after its `StepJoint`
  asset — `02-Cart.Slider.joint` and `04-Pole.Hinge.joint` in
  `asy-cart-pole.xml`. A `StepJointSet` holds `stepAssetPath` and its
  `joints`. `StepJointType` is `Slide = 0`, `Hinge = 1`.
- **The cart-pole's joint set** `Assets/Simulation/MuJoCo/01-Cart-Pole.root.asset`
  (a `StepJointSet`): `joints` = `02-Cart.Slider.joint` (Slide, body
  path `[4]` → `asy-cart`, `pos` (0,0,0), `axis` (1,0,0)) and
  `04-Pole.Hinge.joint` (Hinge, body path `[4, 53]` → `asy-pole`,
  `pos` (0,0,0), `axis` (0,0,−1)), both with `root` pointing back at
  the set; `actuators` = `03-Cart.Slider.Actuator`. Its
  `stepAssetPath` was stale (`Assets/asy-cart-pole.step`); the user
  updated it on 2026-10-22 to `Assets/Simulation/Robot/asy-cart-pole.step`,
  the STEP asset's actual location and the loop scene's instance source
  (uncommitted).
- **The hinge angle's sign flips between MuJoCo and Unity.** The asset
  axis (0,0,−1) is exported as MJCF `axis="0 -1 0"`: the Unity↔MJCF
  map swaps Y and Z, a mirror (determinant −1), and a mirror reverses
  rotation sense. The same physical swing that is `+qpos` about the
  MJCF axis is `Quaternion.AngleAxis(-qpos°, mj.axis)` in Unity.
  Worked check: MuJoCo +90° about Z takes X to Y; in Unity coordinates
  that is X to Z about Y, and `AngleAxis(+90, up)` takes X to −Z, so
  the angle must be −90°. For this joint the readable equivalent is
  `Euler(0, 0, +qpos°)` about the pole's local Z. Slides have no sign
  issue (a translation maps straight through the swap).
- **MuJoCo's angles are radians at runtime** — "mjModel always uses
  radians" (MuJoCo 3.15.0 XML reference, `compiler/angle`); the
  compiler's `angle` attribute only governs angles written in the XML.
- **Finding the STEP instance at runtime** (precedent,
  `SyntheticDataGenerator.FindStepAssetInstance` and
  `MarkerManager.ResolveRobot`): in the editor, a scene root whose
  prefab source path is the STEP asset path; in a player, a root named
  after the STEP asset (file name without extension).
- **Markers come from `RigSO.Apply(assemblyInstance)`**
  (synthetic-data-gen): it instantiates each `Marker_N_<label>` quad
  under its anchor's sibling path, disables the anchor renderers, and
  applies the texture flips. An SO cannot run itself:
  `SyntheticDataGenerator` calls `m_Rig.Apply(instance.gameObject)` at
  startup. The older `MarkerManager` component is used by no scene or
  prefab. The cart-pole's rig asset is
  `Assets/Simulation/Robot/Rig.asset`.
- **The loop scene** `Assets/SimulationLoop/Scenes/SimulationLoop.unity`
  (user-made copy of `SampleScene` with the generator objects removed):
  the `asy-cart-pole` STEP instance at the origin with no overrides
  beyond its name and root transform; `Main Camera` (physical camera:
  focal length 6 mm, sensor 4.968×3.726 mm, gate fit mode 3) at
  (0, 0.25, −2.41), no rotation; a Directional Light; the Global
  Volume; skybox `neon_photostudio_4k`.
- **The dataset generator's frame** (`GenerationSetRunner.cs`,
  synthetic-data-gen package): renders the capture camera into
  `RenderTexture(w, h, 24)` at the set's `CameraImageWidth/Height` —
  **1456×1088** in all ten `Set-NN.asset` files — reads back `RGB24`,
  then `GreyscaleJpeg`: per pixel `l = 0.299 r + 0.587 g + 0.114 b`,
  written back as equal RGB channels, `EncodeToJPG(90)`. That function
  is `private static` in an `internal` class. The `24` is the depth
  buffer's bit count — what lets near surfaces hide far ones.
- **Package precedent:** `unity/projects/packages/simulation.cart-pole.synthetic-data-gen`
  — `package.json` (`"unity": "6000.7"`), `Runtime/` and `Editor/`
  asmdefs named `Simulation.CartPole.SyntheticDataGen.*`, referenced
  from `Packages/manifest.json` as
  `"file:../../packages/simulation.cart-pole.synthetic-data-gen"`.
- **Player settings:** the project-wide `ProjectSettings.asset` still
  reads `productName: My project`, `runInBackground: 0`,
  `fullscreenMode: 1`. The loop's player is built from the build profile
  `Assets/Settings/Build Profiles/Windows (Loop).asset` (user-made,
  uncommitted), whose player-settings override sets
  `productName: SimulationLoop`, `runInBackground: 1`, and
  `fullscreenMode: 3` (windowed). The project's build scene list has
  `SampleScene` disabled and `SimulationLoop` enabled. The only quality
  level (`PC`) has `vSyncCount: 0`; `Build/` is gitignored
  (`unity/projects/cart-pole/.gitignore:5`).
- **Rust toolchain on this PC:** cargo 1.99.0 / rustc 1.99.0; `aruco/`
  is the repo's existing crate precedent (edition 2024).

## Settled decisions

- Package `simulation.cart-pole.sim-loop`, namespace
  `Simulation.CartPole.SimLoop`, beside synthetic-data-gen. It starts
  as a skeleton; earlier project scripts are pulled in only as needed.
  Neither `JointDriver` nor `StateServer` is pulled in:
  `SimulationLoopController` replaces both, reusing `JointDriver`'s
  drive math (with the hinge sign corrected) and `StateServer`'s
  render-and-queue pattern.
- **`SimulationLoopController`** (MonoBehaviour) takes three assets —
  the `StepJointSet`, the hinge `StepJoint`, and the slide `StepJoint` —
  plus the `RigSO` (whose `Apply` it calls on the instance at startup,
  as the generator does) and the render camera. `OnValidate` enforces that the hinge is a
  Hinge in the set and the slide a Slide in the set. It drives those
  parts, renders the frame, and owns the player's WebSocket connection.
- **The hinge sign is a Unity↔MJCF constant**, hard-coded as
  `const int MjcfToUnityHingeSign = -1` with its derivation (*Facts*) —
  a property of the frame mapping, not of any joint, so it is not a
  joint-asset field.
- **The orchestrator launches the player** with its address as a
  startup argument — `-orchestrator ws://<host>:<port>` — and the
  player connects to it. The orchestrator is the WebSocket server; the
  player is a client. *Superseded (`synthetic-loop-dia.md`):* the
  orchestrator still launches the player, now with `-orin ws://ADDR`;
  the player connects to the synthetic Orin's frame socket, and the Orin
  tells the orchestrator once it has registered.
- **Messages:** orchestrator → player, one **text** message per state:
  `{"id": 7, "joints": [{"name": "02-Cart.Slider.joint", "qpos": 0.05}, {"name": "04-Pole.Hinge.joint", "qpos": 3.14}]}`
  (MJCF joint names; meters / radians). Player → orchestrator, one
  **binary** message per state: the frame's JPEG bytes. One reply per
  request, in order — lockstep. *Superseded (`synthetic-loop-dia.md`):*
  the same two messages pass between the synthetic Orin and the player:
  the Orin sends `{"id", "joints"}` (the trigger), and the player
  returns the JPEG to the Orin.
- **Orchestrator stub** in `orchestrator/` (Rust): launches the player
  (or, with `--no-launch`, waits for editor Play mode to connect),
  sends a scripted sweep or one fixed pose, saves and checks the
  frames. It stands in for MuJoCo; it is the seed of the real
  orchestrator, not the orchestrator plan. *Superseded (2026-10-23,
  `synthetic-loop-dia.md` 6–13):* the player connects to the synthetic
  Orin's frame socket (`-orin`), so the orchestrator has no player
  socket: the sweep and `--pose` are gone, the Orin triggers every frame
  and saves it, and `--no-launch` now means editor Play mode connects to
  the Orin (`m_EditorOrinUrl`, default `ws://127.0.0.1:9001`).
- Frame contract: 728×544 greyscale JPEG, q90 (`synthetic-loop-dia.md`, *Contracts*).
- Loop scene: `Assets/SimulationLoop/Scenes/SimulationLoop.unity`.
- **SO pattern, as synthetic-data-gen does it:** what the loop controls
  is identified by ScriptableObjects carrying sibling-index paths — here
  the existing `StepJointSet` and its `StepJoint` assets. The STEP
  instance and parts are resolved at runtime; **nothing is authored on
  the STEP instance** — no components, no scene references into it, no
  serialized fields pointing at its nodes.
- Human-facing angles (logs, readouts, comments) are Euler degrees; the
  drive computes from the asset's `mj.axis`.

## Requirements and recommendations

Requirements (the loop is wrong without them):

- **Q1 — same greyscale as the training frames.** The loop's frames
  must use the generator's exact conversion (BT.601 luma, equal RGB
  channels, q90), or the detector sees a different image domain than
  it trained on.
- **Q2 — keeps serving when unfocused.** The player must process
  messages while another window has focus (`runInBackground = 1`).
- **Q3 — connects where it is told.** The player takes the
  orchestrator's address from `-orchestrator`; a player started
  without it logs an error and quits. In the editor, an inspector
  field supplies the address instead (default `ws://127.0.0.1:9000`,
  the stub's default). *Superseded (`synthetic-loop-dia.md`):* the
  address it is told is the synthetic Orin's frame socket, given as
  `-orin`.
- **Q4 — hinge sign.** The hinge is driven with `-qpos` about
  `mj.axis` (*Facts*); `JointDriver`'s `+qpos` would swing the pole
  the opposite way from MuJoCo.

Recommendations (choices, with the reason):

- **R2 — one greyscale implementation.** Move the luma conversion into
  one public helper in synthetic-data-gen and call it from both the
  generator and the controller. A copy in sim-loop would satisfy Q1
  only until one of them changes.
- **R3 — joints only, no `rig` payload.** The loop sends MuJoCo's
  `qpos` by MJCF joint name, applied in MJCF semantics. The old `rig`
  payload drove the same parts through `MarkerManager`'s own axis
  convention; two conventions in one loop invite a sign mismatch.

## Steps

1. **Package skeleton — done 2026-10-22** (compiles; the player built).
   `unity/projects/packages/simulation.cart-pole.sim-loop/` with
   `package.json` (`version` 0.1.0, `displayName` "Cart-Pole Sim Loop",
   `unity` "6000.7") and `Runtime/Simulation.CartPole.SimLoop.Runtime.asmdef`
   (`rootNamespace` `Simulation.CartPole.SimLoop`, references
   `com.greyhound.step.runtime`);
   `"simulation.cart-pole.sim-loop": "file:../../packages/simulation.cart-pole.sim-loop"`
   in `Packages/manifest.json`. Step 3 adds the
   `Simulation.CartPole.SyntheticDataGen.Runtime` reference.
   No `.meta` files are authored — Unity mints them. *Check (user):*
   the editor imports the package and compiles clean.
2. **`SimulationLoopController` — joints. Done 2026-10-22**
   (`Runtime/SimulationLoopController.cs`; compiles, and the built player
   drives both parts — *Results*).
   - **Input:** `m_JointSet` (`01-Cart-Pole.root.asset`), `m_Hinge`
     (`04-Pole.Hinge.joint.asset`), `m_Slide`
     (`02-Cart.Slider.joint.asset`), `m_Rig` (`Rig.asset`),
     `m_RenderCamera` (`Main Camera`).
     No sim-loop SO is needed: each `StepJoint` already holds its part's
     sibling-index path (`body`, bound by `StepSiblingPathDrawer`'s drop
     zone in the joint's own inspector) and its MJCF semantics
     (`mj.type`, `mj.axis`, `mj.pos`).
   - **Validation:** `OnValidate` warns, and `Start` (the only check a
     built player runs) errors and stops, on: a missing asset or camera;
     a hinge that is not a Hinge or not in the set; a slide that is not
     a Slide or not in the set; a joint name the MJCF exporter's
     sanitizer would change (any character outside `[A-Za-z0-9_.-]`) —
     the asset name is the wire key and must equal the XML's
     `<joint name>`.
   - **Instance and parts:** the STEP instance is found by
     `SyntheticDataGenerator`'s rule — in the editor a scene root whose
     prefab source path is `set.stepAssetPath`, **else** a root named
     after that asset's file name (in a player, always the name). The
     name fallback also covers a stale `stepAssetPath` (the instance is
     named after the STEP file). `m_Rig.Apply(instance.gameObject)`
     then adds the marker quads (runtime only). Each part is resolved
     through its
     joint's `body` path; rest poses are captured in the parent's space
     at `qpos = 0`, so the pole follows the cart — all in memory,
     nothing written back.
   - **Drive:** `Apply(jointName, qpos)` routes by asset name. Slide:
     rest position + rest rotation · (`mj.axis` · `qpos`). Hinge:
     rotate by `AngleAxis(MjcfToUnityHingeSign · qpos°, mj.axis)` in
     the part's frame, shifted so the pivot `mj.pos` stays put (Q4;
     0 = hanging, π = upright). Local values are meters because
     greyhound bakes the import scale into vertices and leaves every
     transform at unit scale. An unknown joint name is warned once and
     ignored.
   *Check (user):* compiles clean; in Play mode both parts resolve (no
   startup errors or warnings) and the STEP instance shows no modified
   fields afterwards. *Status:* compile and player run pass (no errors
   or warnings in `Player.log`); the editor Play-mode half is not yet
   checked.
3. **Shared greyscale helper (R2). Done 2026-10-22** (compiles; the
   loop's frames use it). Not yet checked: one generator run. `GreyscaleEncoder` (public static, synthetic-data-gen
   `Runtime/GreyscaleEncoder.cs`): `Quality = 90` and `Encode(Texture2D)`
   — the generator's former private `GreyscaleJpeg`, moved verbatim.
   `GenerationSetRunner` now calls `GreyscaleEncoder.Encode` and takes
   its `JpegQuality` from `GreyscaleEncoder.Quality`; output is
   byte-identical by construction. The sim-loop asmdef references
   `Simulation.CartPole.SyntheticDataGen.Runtime`. *Check (user):*
   compiles; one generator run's frames still look as before.
4. **`SimulationLoopController` — connection and frame. Done
   2026-10-22** (`ClientWebSocket` works in the Windows player —
   *Results*).
   - **Address (Q3):** `-orchestrator <url>` from the command line in a
     player; `m_EditorOrchestratorUrl` (default `ws://127.0.0.1:9000`)
     in the editor. A player without the argument logs the error and
     quits with exit code 1.
   - **Socket:** `System.Net.WebSockets.ClientWebSocket`, connected on a
     background task that queues every text message; `Update` drains
     the queue on the main thread (Unity APIs are main-thread only —
     `StateServer`'s pattern). All state is instance fields, so the
     statics-analyzer annotation is not needed.
   - **Per message:** parse the JSON (`JsonUtility`, nested
     `[Serializable]` mirrors), `Apply` each joint, render, send the
     JPEG as one binary message. Each send chains on the previous one
     (`ClientWebSocket` allows one send at a time), which also keeps
     replies in request order.
   - **Frame (Q1):** render the assigned camera into
     `RenderTexture(728, 544, 24)`, read back `RGB24`,
     `GreyscaleEncoder.Encode`.
   - **Close:** when the orchestrator closes the socket, a player quits
     with exit code 0; a failed connection quits it with 1. In the
     editor, Play mode stays up and only the driving stops.
   *Check (user):* compiles clean. `ClientWebSocket` is expected to
   work in the Windows desktop player and the editor; the step-8 run
   confirms it.
5. **Loop scene — done 2026-10-22.** `Assets/SimulationLoop/Scenes/SimulationLoop.unity`
   exists (STEP instance, physical camera, light, volume, skybox; see
   *Facts*). The user adds, on a new GameObject **outside** the STEP
   instance, a `SimulationLoopController` with `01-Cart-Pole.root.asset`,
   `04-Pole.Hinge.joint.asset` (hinge), `02-Cart.Slider.joint.asset`
   (slide), `Rig.asset`, and `Main Camera` assigned. No `MarkerManager`:
   the controller applies the rig's markers itself (step 2).
   *Check (user):* the STEP instance has no added components and no
   overrides beyond those listed in *Facts*.
6. **Player settings — done 2026-10-22** through the
   `Windows (Loop)` build profile's override (*Facts*): `runInBackground`
   on (Q2), windowed, product name `SimulationLoop`; the build scene
   list enables only `SimulationLoop`. `vSyncCount` is already 0, so
   frame rate is not capped by the display.
7. **Build — done 2026-10-22:** Windows x64 player at
   `unity/projects/cart-pole/Build/SimulationLoop.exe` (gitignored;
   `Build/` also still holds the older `My project.exe`). Player log:
   `%USERPROFILE%\AppData\LocalLow\DefaultCompany\SimulationLoop\Player.log`.
8. **Smoke test with the stub** (see *Orchestrator stub*):
   - editor first: start the stub with `--no-launch`, then enter Play
     mode;
   - then the build: start the stub with `--player <exe>`.
   *Pass:* every frame prints `728x544`; the saved frames are grey and
   show the markers (Done-when 1–3, 5). *Status:* the build half passed
   2026-10-22 (*Results*); the editor half (Done-when 5) is not yet run.
9. **Convention check against MuJoCo — passed 2026-10-22** (*Results*). Send one pose with
   `--pose 0.2,0.5`; set the same values in `simulate`'s Joint panel on
   `asy-cart-pole.xml`. *Pass:* cart on the same side, pole tilted the
   same way (Done-when 4). A mismatch is a `SimulationLoopController`
   sign/axis bug, fixed before anything consumes frames.

## Orchestrator stub (`orchestrator/`)

A Rust binary crate, member of the repo-root Cargo workspace
(`Cargo.toml`: shared `[workspace.package]`, `[workspace.dependencies]`,
and `[workspace.lints]` — the greyhound-snake workspace pattern; the
superseded `aruco/` crate is excluded and untouched). `target/` is
gitignored. Run with `cargo run -p orchestrator -- <options>`.

Written and tested 2026-10-22: against a stand-in WebSocket client (a
728×544 reply passes, a 1456×1088 reply is flagged and fails the run,
`--pose -0.2,0.5` sends one state, a launched program that exits before
connecting fails at once), then against the built Unity player
(*Results*).

- **Serves** a WebSocket on `--listen` (default `127.0.0.1:9000`).
- **Launches** the player given by `--player <exe>` with
  `-orchestrator ws://<listen>`, then waits for it to connect
  (`--connect-timeout`, default 120 s, covering player startup). With
  `--no-launch` it launches nothing and waits for any client — editor
  Play mode.
- **Sends** either a sweep of `--frames` states (default 60): cart
  `0.4·sin(2πi/N)` m — inside the ±0.4 m `ctrlrange` — and pole
  `2πi/N` rad, one full turn starting from hanging; or, with
  `--pose <cart>,<pole>`, that single state.
- **Receives** one binary message per state (`--reply-timeout`, default
  10 s), reads width, height, and channel count from the JPEG's SOF
  header, prints them with the state (pole also in degrees), and writes
  `frame_NNNNNN.jpg` into `--out` (default `target/orchestrator-frames`).
  Any frame that is not a 728×544 JPEG fails the run (exit code 1)
  after all frames are checked.
- **Ends** by closing the socket and stopping the player process.
- Joint names default to the cart-pole's MJCF names
  (`--cart-joint`, `--pole-joint` override them).

## Not in this plan

The real orchestrator (MuJoCo stepping, T/D scheduling, the other
legs), detector, headless/batch-mode running, pooling, per-episode
environment randomization, the remaining project-level scripts (pulled
in only when a later plan needs them).

## Placeholders

- **Camera pose** — the scene's (0, 0.25, −2.41), no rotation, is
  nominal; the real mount's measured pose replaces it.
- **The real camera's greyscale** — the IMX296 is a colour sensor; the
  robot's reduction to 728×544 greyscale must use the renderer's method
  (Q1), or the real frames will not look like the training frames.
- **Environment** — the scene's `neon_photostudio_4k` skybox and
  Directional Light, no desk; replaced when the real bench's look is
  chosen.
