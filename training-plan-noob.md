# The Beginner's Plan — Training the YOLO Detector, Step by Step

This is `training-plan.md` translated for someone who has never trained a
neural network. Same plan, same facts — but every step says exactly where to
click, what to type, and what you should see. When a step is done, it says
**Pass:** — if you don't see that, stop and use the Troubleshooting appendix.

Companion document: `training-plan.md` holds the deeper rationale (why each
choice was made). You do not need it to execute this plan.

Field placement, now consistent in both documents: `positionJitter`,
`jitterSeed` and `robotBounds` live on the **capture component**
(`RigStateCapture`), not the randomizer — verified in the landed source.

---

## Part A — What you are actually doing

In plain words, the whole project is three sentences:

1. **Unity makes the photos.** The editor "plays" a scene of a cart on a rail
   with a swinging pole thousands of times. Every play renders one photo and
   writes one text file listing where each of the four ArUco markers is in
   that photo (a rectangle per marker). Photo + text = one training example.
2. **YOLO learns from them.** A program called Ultralytics YOLO looks at
   thousands of those pairs and adjusts itself until it can predict the
   rectangles from a photo it has never seen.
3. **The Pi uses the result.** The trained model is exported to a single
   `.onnx` file; a Raspberry Pi 5 with a camera runs it and sees the markers
   in real life.

The rectangles YOLO predicts are called **boxes**. The four markers are
called **classes** and always have these fixed numbers (never change this
order anywhere):

| class number | name  | what it is |
|---|---|---|
| 0 | Left  | marker tag 376, left side of the cart |
| 1 | Right | marker tag 596, right side of the cart |
| 2 | Pivot | marker tag 920, the pole's hinge |
| 3 | End   | marker tag 937, the pole's free end |

## Part B — Before you start

### Hardware (the honest part)

Training needs an **NVIDIA graphics card**. Check what you have: open
PowerShell and run `nvidia-smi`. If it prints a table — good, note the name
and the memory (e.g. 12 GB). If it prints "not recognized" you have no
usable NVIDIA GPU, and training at full resolution on the CPU takes weeks —
do not start Phase 4 until you have one (borrow, buy, or rent one).

You also need **~50 GB of free disk** for a full-size campaign (30,000
frames ≈ 3 GB committed corpus + ~35 GB of temporary sidecars — an estimate
from the retired 7-frame corpus; re-measure from the first docs-on run).

### Where to train (Windows? Linux? Mac?)

The rule: **train on whatever machine has the NVIDIA GPU — the OS itself
barely matters.** In order of preference for this repo:

1. **Windows native** (the default, and what this plan's commands assume).
   The repo, the Unity capture pipeline, and `uv` already live here.
2. **Native Linux** — only if a separate machine has a bigger GPU. Most
   standard ML environment. Two couplings to handle: the dataset (Phase 3
   output) must be copied over, and `cart-pole.yaml`'s `path:` is an
   absolute Windows path (`D:/git/cart-pole/...`) that must be rewritten
   with that machine's path or training silently fails to find the data.
3. **Windows/WSL2** — works (the NVIDIA driver passes through), but one
   real trap: the repo sits on `/mnt/d/...`, and reading thousands of JPEGs
   per epoch across the Windows↔Linux file boundary starves the dataloader.
   If WSL is used, copy the dataset **into** the WSL filesystem first.
4. **macOS** — last resort. No CUDA; Apple Silicon trains via
   `device=mps`, which is slower and far less exercised than CUDA. Fine for
   a small pilot, not the campaign.

Two things no choice changes: **capture stays in the Unity Editor on this
Windows machine** — that is where the dataset-generation pipeline runs,
full stop. And the exported `.onnx` file is OS-independent — the Pi 5 runs
it on its ARM CPU no matter where training happened, so the training
machine's OS has zero effect on deployment.

Practical rule: don't move training off Windows for OS reasons — only for
GPU reasons. A better GPU elsewhere is the one motive that pays for the
dataset-copy overhead.

### Software

- The Unity 6 project at `unity/projects/cart-pole/` opens without red
  errors in the Console.
- `uv --version` works in PowerShell (the Python tool used in Phases 3–4;
  this repo's rule: always `uv`, never `pip`).
- An internet connection for Phase 4: the training library (a multi-GB
  download through `uv`) and the pretrained starting weights (~5 MB, fetched
  automatically on first use) both come from the network.

### Words you will meet

| Word | Meaning here |
|---|---|
| Play mode | Unity running the scene live (the ▶ button at the top). |
| Hierarchy | Unity panel listing everything in the scene by name. |
| Inspector | Unity panel showing the selected object's settings. |
| Project window | Unity panel showing the files on disk (Assets). |
| Console | Unity panel printing log lines — your run's heartbeat. |
| SO / asset | A settings file you create via right-click → Create → … |
| Seed | A number that makes randomness repeatable. Same seed = same dataset. |
| Epoch | One full pass of the trainer over your dataset. |
| mAP50 / mAP50-95 | Accuracy scores: how well predicted boxes match the truth. Higher is better; 1.0 is perfect. |
| `wip~` folder | A run's scratch sidecar — large, temporary, git-ignored. |

### The two rules that are never optional

1. **Never edit a `.txt` label by hand.** Not to fix a bad box, not to
   delete one. A broken dataset is fixed by regenerating, never by editing.
2. **A run you intend to train on must finish completely.** Only after the
   run's last Console line does the pipeline write its verification receipt.
   Stopping Play early leaves a run the dataset builder will refuse.

---

## Phase 0 — Check the scene once (about 30 minutes)

Everything below happens in the Unity editor. Open the project, wait for the
bottom-right spinner to finish, and look at the **Console** — it must show no
red lines.

The landed scene is **not** yet in this plan's target state: both debug
toggles are off, the capture holds `positionJitter` 0.5 / `jitterSeed` 0 /
`robotBounds` unassigned, the randomizer holds `states` 1 / `seed` 0, the
desks list wires 4 of the 5 DeskSOs (Executive missing), and three
frame-reproduction situations (`Situation-01/02/03`, mined from a retired
run) are already in `forcedSituations`. Phases 0–1 bring the scene to
target.

### 0.1 The scene exists and is wired

1. Project window → `Assets/Scenes` → double-click `SampleScene`.
2. Click around in the Hierarchy until you find the object that carries the
   `MarkerManager` component (selecting objects shows their components in
   the Inspector). Check its Inspector:
   - `rig` points at `Assets/Rig.asset`,
   - `robot` points at the cart-pole assembly,
   - `applyOnStart` is on.

   **Pass:** press ▶ for a few seconds, look at the Hierarchy under the
   assembly — you should see children named `Marker_0_Left`, `Marker_1_Right`,
   `Marker_2_Pivot`, `Marker_3_End`. Press ▶ again to stop.

### 0.2 The capture camera

Find the object carrying `RigStateCapture` (there is only one). Its
Inspector must show:

- `captureCamera` assigned (a Camera),
- `captureWidth` = **1440**, `captureHeight` = **1080** — the code default
  is 960×540, so the scene must be holding the bigger values; if you see
  960/540, set them to 1440/1080,
- `outputDirectory` = `Assets/Captures/cart-pole`,
- `jpegQuality` = **90** (leave it — the real camera's frames must match the
  training photos' compression exactly),
- `generateDebugImages` = **on**, `generateDebugDocs` = **on** (see why in
  Phase 2 step 5 — turning docs off makes a run unverifiable; the landed
  scene holds both off, which is why the current corpus has no receipt),
- `robotBounds` assigned (a `RobotBounds` reference; the cube exists in the
  scene — Phase 0.3; the landed component leaves it unassigned, covered
  only by the code's find-fallback),
- `positionJitter` = 0.15 (the code default; the landed scene serializes
  0.5), `jitterSeed` = 2001 (any nonzero; the landed scene holds 0 = fresh
  randomness each run).

On that same camera object, the **Camera** component itself must be a
physical camera matching the real rig (sensor gate 4.968 × 3.726 mm,
focal length 6 mm) and be tagged `MainCamera`: in the Inspector's top line,
the **Tag** dropdown must show `MainCamera`. If not, set it — the
front-light guarantee silently breaks without it (sun ends up behind the
rig in some frames).

**Pass:** the written frames (Phase 1) come out 1440×1080 — you can check a
frame's size by right-clicking it in Explorer → Properties → Details.

### 0.3 The bounds cube

The `RobotBounds` cube defines where the camera aims and how far it jitters.
Select it in the Hierarchy and confirm its box visually encloses the whole
rail-and-pole workspace as seen from the camera.

**Pass:** during the Phase 1 run, the Console prints a jitter offset per
frame (and no "no RobotBounds cube" warning).

### 0.4 The pools (what Unity can randomize)

The randomizer picks from pools. The landed inventory: 5 desks
(`Assets/Cart-Pole/Props/Desks/`), 3 HDRI skyboxes
(`Assets/Cart-Pole/Vision/HDRI/`). Adding more is optional now and becomes
the growth lever in Phase 2:

- **Desks:** drop `.fbx` files into `Assets/Cart-Pole/Props/Desks/`, then
  top menu bar → **Cart-Pole → Register Desks** (new DeskSOs land in
  `Assets/Cart-Pole/Desks/`). Select each new desk asset and tune its
  position/rotation/scale in the Inspector, then add it to the randomizer's
  `desks` list (0.5 below). The landed list wires 4 of 5 —
  `Desk (Executive)` is not in it.
- **HDRIs:** drop `.exr` files into `Assets/Cart-Pole/Vision/HDRI/`, then
  top menu bar → **Cart-Pole → Import HDRIs as Skyboxes**, then add the new
  skybox materials to the randomizer's `skyboxes` list.

### 0.5 The randomizer's wiring

Select the object carrying `RigStateRandomizer`. Its Inspector must show:
`markerManager` assigned, `desks` list filled, `skyboxes` list filled,
`directionalLight` assigned (the scene's sun).

### 0.6 The one-keypress sanity check

Right-click the **header** of the `RigStateRandomizer` component (its title
line in the Inspector) → click **Randomize Once**.

**Pass:** the cart/pole jump somewhere new, and the desk, background, and
light change visibly. Each click changes everything once more. This writes
**nothing** — it is a visual check only.

Also do not use **Cart-Pole → Dataset Generator** — that window belongs to
an older, separate pipeline and is not part of this plan.

---

## Phase 1 — The pilot: one small run end to end (an evening)

Goal: a 300-frame run that passes the quality gate, so you have seen every
step before scaling.

### 1.1 Create the forced situations

These are six "hard cases" that every future run will always include at the
start. Create them once:

1. Project window → right-click inside a folder (suggest a new
   `Assets/Cart-Pole/Situations/` folder) → **Create → Cart-Pole → Forced
   Situation**. Unity creates a new asset; give it the file name from the
   table below.
2. Select it and fill the Inspector fields exactly as the table says
   (distances in meters, angles in radians — the fields say so too).
3. Repeat for all six.

| File name | label | cartPosition | poleRotation | other fields |
|---|---|---|---|---|
| cart-left | cart-left | −0.45 | 0 | — |
| cart-right | cart-right | 0.45 | 0 | — |
| pole-extreme-left | pole-extreme-left | 0 | −3.0 | — |
| pole-extreme-right | pole-extreme-right | 0 | 3.0 | — |
| light-dim | light-dim | 0 | 0 | `useLight` on, `lightIntensity` 2.0 |
| light-bright | light-bright | 0 | 0 | `useLight` on, `lightIntensity` 4.0 |

For the two light situations, `lightYaw` needs a realistic value. No landed
manifest exists (the landed corpus was captured docs-off); the landed
Situation assets hold −23.7 / 75.5 / 33.8 — use one of those until the first
docs-on run produces manifests to copy from.

Two optional extras, only if you want them: a situation can also freeze the
camera (`useCameraPosition` on, plus `cameraPosition`/`cameraRotation`
copied from a manifest — reproduces one exact shot), and force a specific
desk or skybox (`useDesk`/`useSkybox` on, then drag the asset in).

4. Select the `RigStateRandomizer` object again, find the `forcedSituations`
   list, click **+** six times, and drag each situation asset into a slot.
   **List order = capture order.** The list already holds three
   frame-reproduction situations (`Situation-01/02/03`, from a retired run)
   — keep or remove them, but count them: with all nine, the forced frames
   are 0–8 of every run.

**Pass:** the list shows the six new entries in the order of the table,
plus the three existing ones.

### 1.2 Set the pilot numbers

`RigStateRandomizer` Inspector:

| Field | Value | Why |
|---|---|---|
| states | 300 | enough for the gate, not yet a training set |
| interval | 0.5 | you are watching it run |
| seed | 1001 | any nonzero number; 0 = new randomness each run |
| cartRange | 0.45 | the rail's physical limit |
| poleSpan | 3.0 | the authored swing parameter |
| randomizeDesks / randomizeSkybox / randomizeLight | on | full environment randomization |

`RigStateCapture` Inspector (same or sibling object):

| Field | Value | Why |
|---|---|---|
| positionJitter | 0.15 | the code default (the landed scene serializes 0.5) |
| jitterSeed | 2001 | distinct from `seed`; keep the pair per run |
| generateDebugImages / generateDebugDocs | on | the verification receipt |

### 1.3 Run it

Press ▶ and then **do not touch the editor**. The Console prints, in order:

1. a plan line (`plan: <forced count> forced + 300 random states, forced
   first` — 9 when the three landed situations stay),
2. a jitter line per frame,
3. `done - 9 forced + 300 random states applied` (matching the plan line),
4. a verification summary line.

Wait for line 4. Then stop Play (▶ again).

**Pass:** all four lines appeared. Stopping before line 4 means no
verification receipt — the run would be refused later.

### 1.4 The quality gate

In Explorer, open `unity/projects/cart-pole/Assets/Captures/cart-pole/` —
your run is inside a fresh `run_<timestamp>` child folder. Open its
`wip~/verification_stats.json` in a text editor. Read it against:

| Field | What it must be |
|---|---|
| `framesScanned` | the forced list's count + 300 (keeping the landed list's three situations: 309) |
| `labelsConfirmed` | near 4 per frame minus the drops (the retired 7-frame corpus's receipt: 27 confirmed / 1 dropped across 28 boxes) |
| `labelsDropped` | small and scattered — a drop means that marker was behind the camera in that frame, which is legal |
| `writeRefused` | **must be false** — true means the detector saw nothing at all; that is a systemic failure (broken wiring, missing crops) and scaling will not fix it |

**Pass:** all four rows above. If `writeRefused` is true or counts are way
off → Troubleshooting appendix, do not scale up.

### 1.5 Eyeball it before you trust it

Top menu bar → **Cart-Pole → Capture Viewer** and **Cart-Pole → Run
Report**. In the viewer, confirm: boxes sit on markers, each crop decodes to
its expected tag, and desks/backgrounds/light visibly vary between frames.

### 1.6 Write down two numbers

- How long the run took (Play start → the `done` line).
- The run folder's size (right-click in Explorer → Properties).

Nothing in this repo has measured throughput yet — **this** is the
measurement you extrapolate Phase 2 from (e.g. X seconds/frame → how many
frames per overnight run).

---

## Phase 2 — Scale out (days, unattended)

### 2.1 Where runs land

Point `outputDirectory` at a base folder. The behavior is automatic: an
**empty** base is used as-is; a **populated** base gets a fresh
`run_<timestamp>` child per play session. Runs can never mix — you never
need to prepare a folder manually. Suggested naming: one base per config,
e.g. `Assets/Captures/cart-pole-r002-s1002`.

### 2.2 The run matrix

Keep this table in this file (or a sidecar) — one row per run, a fresh
seed + jitterSeed pair every time:

| run | states | seed | jitterSeed | pool delta | notes |
|---|---|---|---|---|---|
| r001 (pilot) | 300 | 1001 | 2001 | — | the QA gate |
| r002 | 1000 | 1002 | 2002 | +2 desks | |
| r003 | 1000 | 1003 | 2003 | +2 HDRIs | |
| … | … | … | … | … | new pair every run |

For bulk runs drop `interval` to 0.05–0.1 — it is pure pacing; the state
application and both renders are synchronous, nothing settles between
states. Start Play, walk away, come back, wait for the summary line.

### 2.3 Grow the variety, not just the count

Frames are cheap; variety is the budget. Past ~10k frames, more desks and
more HDRIs improve real-world transfer more than more frames (this is a
recommendation, not a law). The landed pools (3 skyboxes × 5 desks) are
thin — target 10+ of each before judging transfer (Phase 0.4 explains how).

### 2.4 The artifact toggles, plainly

Two capture toggles control what is written. Both change **storage**, never
verification:

- `generateDebugImages` — keeps the big 8K render + marker crops as files
  (~1.14 MB per frame). Turning it off saves that disk but still verifies.
- `generateDebugDocs` — writes the run's JSON bookkeeping, including
  `verification_stats.json`. **A run for training must keep this on**:
  without the stats file there is no file-level proof the labels were
  verified, and Phase 3 refuses such runs.

`markers.json` (the class map) is always written regardless — without it the
label files' class numbers are uninterpretable.

### 2.5 Disk math (estimate, from the retired 7-frame corpus — re-measure
from the first docs-on run)

30k frames ≈ **3 GB** of committed corpus and ≈ **35 GB** of `wip~`
sidecars. After a run's `verification_stats.json` passes the gate (1.4), you
may delete that run's `wip~` folder — the labels are already confirmed, and
the verifier's refusal rule keeps a later re-verify from ever emptying them.

### 2.6 Get the corpus out of Unity's way

At tens of thousands of frames, Unity re-imports every `.jpg` under
`Assets/` and the editor slows down. After a run is verified and committed:

```powershell
git mv "unity/projects/cart-pole/Assets/Captures/<run>" "datasets/<run>"
git commit -m "move verified run <run> out of Assets"
```

Nothing in the pipeline needs the frames inside Unity anymore once verified.

### 2.7 Commit per run

One run's frames + labels + `markers.json` = one commit. That keeps the
corpus reviewable and the history bisectable.

---

## Phase 3 — Assemble the dataset (an afternoon)

**Boundary:** this phase only copies, splits, and counts. No image
decoding, no ArUco. All Python goes through `uv`.

### 3.1 Prerequisite: the two scripts

The assembly scripts are **specified in `training-plan.md` Phase 3 step 5
but not yet written**. They are:

- `training/scripts/build_dataset.py` — walks run folders, pairs each
  `frame_NNNNNN.jpg` with its `.txt`, splits into train/val, hardlinks the
  files into the layout below, and writes the yaml. It **refuses** a run
  whose `verification_stats.json` is missing or refused — that is the only
  provenance check a pruned corpus can offer, so the stats file must
  survive pruning (or be copied to the run root before pruning).
- `training/scripts/dataset_stats.py` — per-class box counts and box-size
  histogram, pure text parsing.

Request these when you reach this phase; they are written by the agent on
your word (stdlib only — zero installs).

### 3.2 The uv project

```
training/pyproject.toml
```

stdlib only for this phase; nothing to install yet.

### 3.3 The layout and the split

The dataset ends up as:

```
training/datasets/cart-pole/
  images/train/*.jpg      images/val/*.jpg
  labels/train/*.txt      labels/val/*.txt
  cart-pole.yaml
```

There are **two** buckets: `train` (the model learns from these) and `val`
(its accuracy is scored on these — the model never learns from them).
Ratios: ~90/10 for the pilot-scale corpus, ~95/5 at scale. The split is
made **by whole run**, not by mixing frames of one run into both buckets —
frames within a run are near-duplicates of each other and would inflate the
score. Forced situations land in train, with one copy of each in val so
every hard case stays measurable.

(The "test set" you may have heard about is played, in this project, by the
real-camera validation set of Phase 5 — that is why there is no third
bucket here.)

### 3.4 The yaml

Written from the run's `markers.json` — never typed from memory:

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

**Requirement:** that class order is the rig slot order. Never reorder.

### 3.5 Integrity checks (each is hand-checkable)

- every `.jpg` has a `.txt` beside it, and vice versa;
- every label line parses as 5 fields, class ∈ {0,1,2,3}, coordinates in
  [0,1];
- empty label files only where all four markers were genuinely behind the
  camera (expect near zero; a run producing many is investigated, not
  accepted);
- spot-check a few frames are 1440×1080;
- the class histogram: class 3 (End) is expected underrepresented — the
  pole swings wide and exits the frame more often. The fix is more runs,
  never label edits.

---

## Phase 4 — Train (the first real training)

### 4.1 Install the trainer (the permission gate)

This step downloads multi-GB dependencies (PyTorch among them). In this
repo it happens only after you say OK:

```powershell
cd D:\git\cart-pole\training
uv add ultralytics
```

### 4.2 Check the GPU

```powershell
nvidia-smi
```

A table means CUDA is usable and `device=0` below is right. No table → stop
(Phase B above); CPU training at 1440 px is weeks, not hours.

### 4.3 The two training runs

Commands are given as **single lines** because this is Windows/PowerShell
(the backslash line-continuations in `training-plan.md` are `sh` syntax and
error out in PowerShell):

```powershell
uv run yolo detect train model=yolo11n.pt data=datasets/cart-pole/cart-pole.yaml imgsz=1440 batch=-1 epochs=200 patience=50 device=0 project=runs name=cp-1440-base
```

```powershell
uv run yolo detect train model=yolo11n.pt data=datasets/cart-pole/cart-pole.yaml imgsz=640 batch=-1 epochs=200 patience=50 device=0 project=runs name=cp-640-base
```

What each knob means, in words:

- `model=yolo11n.pt` — the smallest YOLO11, starting from publicly
  pretrained weights. **First run fetches the file automatically** (~5 MB
  from Ultralytics' servers) — a normal, expected download.
- `imgsz` — the resolution the model sees. The markers measure 24.0–33.3 px
  wide at 1440; at 640 they shrink to 10.7–14.8 px, which is below the
  detector's comfort zone. That is why 1440 is the base and 640 is the
  one-time comparison (it answers: can the Pi afford native-scale
  inference, or must it upscale?). For the 640 run, also try the
  small-object variant `yolo11n-p2.pt` — it adds the P2/stride-4 head; at
  1440 the standard head stack is fine.
- Train on the frames as written — no re-encoding, no PNG copies. The q90
  JPEG statistics are part of the domain. The augmentation policy
  (`fliplr` on, `degrees`/`perspective`/`copy_paste` off) lives in
  `training-plan.md` Phase 4.
- `batch=-1` — let the tool fit as many images per step as your GPU's
  memory allows. If it crashes with "out of memory", re-run with
  `batch=8` (then 16) — nothing else changes.
- `epochs=200` — up to 200 passes over the dataset; `patience=50` stops
  early if nothing improves for 50 epochs. Early stopping is normal and
  good.
- `device=0` — the first GPU.

While training, the console shows a progress bar per epoch. A run makes a
folder `training/runs/<name>/`.

### 4.4 What to look at afterwards

All results land in `training/runs/<name>/`:

- `results.csv` — one row per epoch; the last row's numbers are the
  summary. The columns with `mAP50-95` are the accuracy scores.
- `weights/best.pt` — the checkpoint to keep (this is what you export).
- `confusion_matrix.png` — open it: the rows/columns are the four classes
  plus background; a bright cell off the diagonal tells you which class is
  being confused with which.

Read the four classes **separately** (the per-class numbers, not only the
overall average): class 3 (End) is expected weakest — an aggregate score
can hide it. Pick checkpoints by val scores, never by how low the training
loss looks.

### 4.5 Comparing the two runs

Same dataset, same val split — so the comparison is fair by construction:
open both `results.csv` files, read the final epoch row of each, compare
per-class mAP50-95. The higher-resolution run should win clearly; if it
does, that settles the Pi's inference-resolution question — record the
answer and move on.

---

## Phase 5 — Export and the real rig

### 5.1 Export to ONNX

```powershell
uv run yolo export model=runs/cp-1440-base/weights/best.pt format=onnx imgsz=1440
```

One file comes out: `best.onnx`. Two facts about it:

- Keep `half=False` (the default) — FP16 helps GPUs, but the Pi runs on CPU
  where it can be slower or unsupported.
- The model expects a **letterboxed** input: the camera's rectangular frame
  is scaled to fit a square with padding added around it. Whatever runs the
  model on the Pi must letterbox exactly the same way Ultralytics does.
  Validate the exported model inside the uv env (needs `onnxruntime`,
  permission-gated install) against a handful of corpus frames before it
  ever touches the Pi.

### 5.2 The closed loop on this machine

The gym/MuJoCo training loop drives the rig through `StateServer`
(`port` 5555): one JSON line in, a JPEG frame back. Its
`captureWidth`/`captureHeight` **must be set to 1440/1080** — the code
default is 960×540, and the model was trained on 1440×1080 q90 JPEGs; the
loop must render at exactly that size and compression.

The current client is `vision/gym_state_sender.py`; a future Rust sender
(greyhound `vision.md` V4 — a plan, not landed) must speak the same wire
format.

### 5.3 The Pi's camera

Lock exposure and white balance on the global-shutter camera. The synthetic
frames have fixed exposure, so the real camera's auto-exposure is a gap you
eliminate on the hardware side for free. Mount the camera to match the
authored physical camera (gate 4.968 × 3.726 mm, 6 mm lens).

### 5.4 The real-world validation set — the actual judge

Synthetic val mAP does not decide readiness; real frames do:

1. collect a few hundred real 1440×1080 frames from the rig across
   poses and lighting,
2. label their marker boxes by hand (a labeling tool — installing one is
   permission-gated),
3. score the exported model against them:

   ```powershell
   uv run yolo detect val model=runs/cp-1440-base/weights/best.pt data=<real-val>.yaml imgsz=1440
   ```

4. the starting bar (a recommendation you may adjust): per-class AP50
   ≥ 0.90, no class below 0.80.

### 5.5 The failure-mining loop — the reason for everything above

Every real-world failure (an occluding desk, a specific pole angle, a light
direction, a cart position) becomes a new **ForcedSituationSO** reproducing
it verbatim (Phase 1.1 shows how; `useCameraPosition` for the shot,
`use*` flags for desk/skybox/light). Once authored, it is in **every**
future run automatically, because forced situations run first. Then:
regenerate the affected runs, rebuild the dataset (Phase 3), retrain
(Phase 4). Worth authoring even without a real failure: the pole fully
horizontal at the rail extremes, a desk overlapping the cart, the End
marker at maximum swing.

The loop to internalize: **runs are cheap — the constraint is environment
variety and failure mining, not frame count.** The Phase 1/2 gate is the
only quality control the corpus has; keep it green before every scale-up.

---

## Appendix — Troubleshooting (what went wrong, most likely)

| You see | Most likely cause | Do this |
|---|---|---|
| `writeRefused: true` in stats | systemic: no crops / broken class map / native bridge not loaded | fix the cause before scaling; never hand-edit labels |
| `framesScanned` ≠ forced + random | a state's capture was skipped | check the randomizer/capture wiring logs |
| boxes ~1.5× too tall | projection ran after the camera target restore | keep the landed capture order; never reorder it |
| sun behind the rig | capture camera not tagged `MainCamera` | tag it (Phase 0.2) |
| no jitter lines in Console | no `RobotBounds` cube wired | place/wire the cube (Phase 0.3) |
| markers gone after a STEP reimport | stale file-ID references | rebind in the Rig inspector |
| detection broke after a package update | old native library still mapped | **full Editor restart** |
| run folders merged | impossible in code | a populated base always gets a fresh `run_<timestamp>` child |
| PowerShell red syntax errors on training commands | backslash continuations pasted from `training-plan.md` | use the single-line commands of Phase 4.3 |
| "CUDA out of memory" mid-training | GPU memory too small for `batch=-1` | re-run with `batch=8` (then 16) |
| `nvidia-smi` not recognized | no NVIDIA GPU / no driver | resolve before Phase 4 (Part B) |
| first training command stalls on a download | it is fetching `yolo11n.pt` / torch | wait; it is a normal one-time fetch |
| labels all empty after a verify | the retired Python checker's wipe failure — cannot happen in the landed code | if you ever see it, treat as systemic; check the verifier wiring |
| editor log confusion | project-local log stale while the global one is live | check both timestamps; grep the project log for `Failed to open 'Logs/Editor.log'` |

## Appendix — Status of claims in this document

- **Source-inspected (read, not executed):** every component behavior cited;
  the menu paths (read from the landed `MenuItem`/`CreateAssetMenu`
  attributes); the ForcedSituationSO, RigStateRandomizer, RigStateCapture,
  and StateServer field names and defaults (both documents place
  `positionJitter`/`jitterSeed` on the capture component); the verify and
  never-wipe semantics; the scene's serialized values.
- **Measured (executed commands):** the landed 4-frame corpus's file
  sizes, label contents, box-width range, frame resolution; pool
  inventory. The 30k-frame disk math is an estimate from the retired
  7-frame corpus.
- **Not executed:** any training, any export, any real-rig validation. The
  throughput, mAP, and sim-to-real numbers here are procedures to obtain
  them, not results. GPU memory behavior of `batch=-1` at 1440 px is
  unknown until the first run.
