# 06 — Generate and evaluate real-world data

**Path: 6 of 7. Before this: 05-POST_TRAINING.md (a validated export).
After this: 07-REFINING.md — read only if the evaluation demands it.**

**Every `uv run` command in this doc (Steps 3, 4, 5):
`cd D:\git\cart-pole\training` first.** `uv run` resolves the Python
environment from the working directory's `pyproject.toml` — from any
other directory it fails or runs in the wrong environment. The copy
step (Step 2) works from any directory: its paths are absolute.

## Purpose

The varied synthetic corpus was built to train the models to
readiness **without real-world training**. This doc is the test of
that premise: auto-label real captures with **both** models and
evaluate what they infer, in the viewer. There is no correction loop
and no retraining here — if the evaluation shows the models labeling
real frames cleanly, the premise is confirmed and this loop is the
whole job. Refinement lives in `07-REFINING.md` and is built only if
the evaluation ever demands it.

**Both models auto-label**, each in its own self-contained dataset:
the **metrology model** (1440) writes the tightest draft, and the
**deployment model** (640) writes what the Pi would generate. The
instrument that proposes is not the instrument being judged — and the
deployment path gets its own record of every frame.

## The two datasets

Each dataset is **pre-named and self-contained** — images, labels,
and its own yaml. The folder name is the model, the model is the
process, and no command ever changes:

```
datasets/real-1440/         the metrology dataset
  data.yaml                 pre-populated legend + pointer
  data/
    images/                 full frames from the capture (Step 2)
    labels/                 1440 auto-labels (Step 5)
datasets/real-640/          the deployment dataset
  data.yaml                 pre-populated legend + pointer
  data/
    images/                 half frames from the same capture (Step 2)
    labels/                 640 auto-labels - what the Pi generates (Step 5)
```

The capture app downloads **both** sizes per session — full frames
and half-sized frames — so the process is **one pass**: full frames
land in `real-1440`, half frames land in `real-640`, **paired by file
stem** (`img_0` in one dataset is the same instant as `img_0` in the
other). Labels are normalized coordinates, so a marker's box is
identical in both datasets — which is what makes the Step 6
evaluation valid across the resolution difference: the 640 is
evaluated at the Pi's operating point.

The `data.yaml` files are **pre-populated** and live at the **dataset
root** — the Ultralytics/DATASETS.md layout. Nothing in them is
dynamic, so they ship with the skeleton and never change between
sessions:

```yaml
train:
- .
val:
- .
names:
- Left
- Right
- Pivot
- End
channels: 1
```

- `names:` is the legend — the rig slot order: 0=Left, 1=Right,
  2=Pivot, 3=End, identical for both datasets because both models
  were trained on that order.
- `val: - .` is the pointer — the dataset root itself; the scan finds
  `data\images\` and pairs each frame with its `data\labels\` file.
  Scoring reads the **1440** yaml only (Step 6); the 640's yaml
  exists so its dataset is self-describing — the 640's labels are
  never scored against themselves.
- `channels: 1` makes the loader convert color frames to greyscale,
  the same conversion the deployment inference does.
- **Placement gotcha (landed 2026-10-18):** a yaml inside `data\` is
  NOT found by the tooling — the labeling GUI could not see the
  labels until the yaml moved up to the dataset root. The yaml
  belongs beside `data\`, never inside it.
- The first labeled set (`run_1792290045-grey`, named before this
  convention) keeps its own yaml at `datasets\real\data.yaml` and
  still scores as-is.
- Every `images\` and `labels\` folder carries a tracked
  `.gitignore` placeholder — the directory skeleton survives clone
  and pull while the data inside stays git-ignored
  (`*` / `!.gitignore`).

Label format — same as the synthetic sets: 5 fields per line
(`class x_center y_center w h`), normalized, one line per visible
marker. Class ids are the rig slot order: 0=Left, 1=Right, 2=Pivot,
3=End.

> ## Step 1 — capture the raw frames
>
> Why: the evaluation must run on the same pixels the on-Pi inference
> consumes — a codec-processed or auto-exposed frame measures a
> different detector than the one that will run.
>
> ```powershell
> rpicam-still -o img_N.jpg --width 1456 --height 1088 ...
> ```
>
> - Raw camera path only — never WebRTC (codec-processed,
>   bitrate-adapted). Ghosting from a streaming path disqualifies a
>   batch.
> - 1456×1088 is the sensor's native output — no rescaling anywhere
>   in the chain.
> - 1 ms shutter, gain/light set for exposure, AE/AWB locked: the
>   synthetic frames have fixed exposure; the real set must match
>   rather than inject a gap.
> - The capture utility — `D:\git\webrtc-pi` (web interface plus a
>   Pi-control GPUI app) — downloads each session as a tar archive
>   and expands it into its own run folder (a `run-<id>` folder in
>   your Downloads, containing the `grey` and `half-grey` variant
>   subfolders) — the expanded folders are Step 2's sources.
> - New camera setup, re-mount, or changed rig position? Run the
>   acceptance check first: `APPENDIX-CAMERA_CALIBRATION.md`.

> ## Step 2 — copy the frames into the datasets (manual)
>
> Why: the repo owns its data. Raw frames must live inside the corpus
> before anything consumes them — a labeling run pointed at a
> Downloads folder reads from volatile ground the repo cannot see.
>
> **Copy — do not move — with Explorer or your file manager:**
>
> The **source** is the utility's expanded run folder — the one
> location that changes between sessions:
>
> - The **full frames** (the `grey` variant folder) →
>   `datasets\real-1440\data\images\`
> - The **half frames** (the `half-grey` variant folder) →
>   `datasets\real-640\data\images\`
>
> - The source folder is the only path in this loop you choose by
>   hand; every destination is literal and never changes.
> - Copy, do not move: the capture folder stays as the app wrote it.
> - Landed (2026-10-18): 147 full frames and 147 half frames from
>   one capture session, landed into the two datasets.

> ## Step 3 — auto-label with the metrology model (1440)
>
> Why: real frames have no geometry to project labels from, so the
> models' inferences are recorded as label files — making them
> viewable, comparable, and checkable instead of fleeting console
> output.
>
> ```powershell
> uv run yolo detect predict model=D:/git/cart-pole/training/runs/cp2-1440-base/weights/best.pt source=D:/git/cart-pole/datasets/real-1440/data/images save_txt save_conf save=True imgsz=1440 project=D:/git/cart-pole/training/runs name=1440
> ```
>
> What the flags produce:
>
> - `name=1440` — the proposals land in
>   `training\runs\1440\labels\`: one 6-field `.txt` per frame
>   (`class x y w h conf` — the confidence column is stripped in
>   Step 5), every detection kept, duplicates included. Reusing the
>   name auto-increments (`14402`, ...) — convert (Step 5) before the
>   next predict run, and read the `Results saved to` line if in
>   doubt.
> - `save=True` also writes annotated JPGs beside the labels — open
>   them to see what the model saw.

> ## Step 4 — auto-label with the deployment model (640)
>
> Why: the 640 is the model the Pi runs — its detections on these
> frames **are** the artifacts the deployment path generates. This
> step records them as label files, so the deployment path's output
> exists as a viewable artifact instead of a fleeting console run.
> The only difference from Step 3 is the model path and the dataset:
>
> ```powershell
> uv run yolo detect predict model=D:/git/cart-pole/training/runs/cp2-640-base/weights/best.onnx source=D:/git/cart-pole/datasets/real-640/data/images save_txt save_conf save=True imgsz=640 project=D:/git/cart-pole/training/runs name=640
> ```
>
> Expected: `training\runs\640\labels\` — one 6-field proposal file
> per frame, same format as Step 3's, feeding the same converter
> (Step 5). The annotated JPGs land beside them.

> ## Step 5 — convert both datasets' proposals into labels
>
> Why: the proposals are 6-field drafts in predict's scratch folders —
> a format the viewer cannot read. The converter turns both datasets'
> drafts into 5-field corpus labels in one command: `real-1440`'s
> labels become the metrology view, `real-640`'s labels the
> deployment view. Full contract: `APPENDIX-CONVERT_PROPOSALS.md`.
>
> ```powershell
> uv run python src\convert_proposals.py
> ```
>
> Expected output — one line per frame per dataset
> (`real-1440 img_3.txt: 4 boxes, classes ['0', '1', '2', '3']`),
> then a per-dataset summary (`real-1440: 147 label files written, 0
> empty, to ...`). Empty labels are legal: the frame's proposals were
> empty. A dataset without images or without proposals is skipped
> with a message naming the missing step; the converter fails (exit
> 1) only when *neither* dataset could be converted.
>
> Convert immediately after Steps 3–4 — before another predict run —
> so the proposals folders cannot desynchronize from the frames.

> ## Step 6 — evaluate
>
> Why: this is the test of the premise. A human looks at what each
> model inferred on real pixels — in the viewer, over both label
> sets — and judges whether the synthetic corpus trained the models
> to readiness.
>
> Open the datasets in the **YOLO Labeler** — the Rust + GPUI tool at
> `D:\git\greyhound-snake\crates\yolo-labeler` — and walk the frames,
> per dataset:
>
> - **`real-1440`** (metrology view): four markers per frame, correct
>   classes, tight boxes. This is the tightest labeling the pipeline
>   can produce.
> - **`real-640`** (deployment view): the same frames through the
>   model the Pi runs, at its 640 operating point on the half-sized
>   frames. Does it see what the 1440 sees?
>
> What **clean** looks like: every frame shows all four markers
> (Left, Right, Pivot, End), one box each, correct classes, no
> phantoms. Landed first look (2026-10-17, `img_3.jpg`): exactly that
> — including Right on the rail-right marker, the class the old
> corpus's model got wrong on most real frames.
>
> **Landed (2026-10-18):** the full 146-frame half-scale deployment
> batch walked in the viewer — every auto-label correct, 100%. The
> ledger: the full-sweep pose (4/4 markers), both parked occlusions
> (pole tip over each rail marker — correct abstention, no phantoms,
> no class confusion at the overlap), human-in-frame noise ignored.
> No systematic divergence; `07-REFINING.md` was not demanded.
>
> What **systematic** looks like: the same mistake recurring across
> frames — one class consistently wrong, one marker consistently
> missed, phantom boxes in consistent positions. Isolated mistakes
> are noise; systematic ones are signal — and signal is what
> `07-REFINING.md` exists for.
>
> Optional numeric check — the two datasets' agreement, scored:
>
> ```powershell
> uv run yolo detect val model=D:/git/cart-pole/training/runs/cp2-640-base/weights/best.onnx data=D:/git/cart-pole/datasets/real-1440/data.yaml imgsz=640
> ```
>
> This scores the 640's frames against the 1440's labels — a
> model-vs-model agreement number at the deployment operating point.
> It is not truth-grounded until the labels are human-corrected
> (07-REFINING.md); treat it as a cross-check, not a verdict.

## What the evaluation means

**Landed (2026-10-18): the 640 deployment batch passed 146/146** —
every auto-label on every half-scale real frame correct, including
both parked occlusions and human-in-frame noise. The premise is
confirmed at full batch scale: the varied synthetic corpus trained
the deployment model to real-world accuracy with zero real-world
training. The deployment path's own labels, sitting next to the
metrology view, are the standing evidence.

The remaining half of the evaluation is the `real-1440` walk — the
metrology view at full scale — for completeness; the deployment-critical
verdict is the one above.

Systematic divergence is not a failure of the premise — it is the
signal the premise's safety net exists for: `07-REFINING.md` holds
the refinement ladder, cheapest step first, built out only as far as
the evidence demands.
