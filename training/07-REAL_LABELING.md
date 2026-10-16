# Real-frame labeling — from raw capture to a scored test set

The process that turns a folder of raw frames from the physical rig
into a labeled real-frame test set, using the trained detector as a
pre-labeler and a human as the class arbiter. Everything here runs
headless except the human review.

## The contract

Real frames are labeled in the **same corpus format as the synthetic
sets**: `img_NNNN.txt` beside each frame, 5 fields per line
(`class x_center y_center w h`), normalized, one line per visible
marker. Class ids are the rig slot order: 0=Left, 1=Right, 2=Pivot,
3=End.

The difference from the synthetic process: there is no geometry to
project, so the **detector writes the first draft** and a human
corrects it. The detector used for proposals is the **1440 metrology
model** (`cp-1440-base` at `imgsz=1440`) — the tightest boxes any of
our models produce — while the model being *evaluated* is the 640
deployment model. The instrument that proposes is not the instrument
being judged.

## The pipeline

```
raw frames (1440x1080, rpicam-still, AE/AWB locked)
        │  yolo detect predict  (cp-1440-base, imgsz=1440,
        │                        save_txt save_conf)
        ▼
runs\real-preview\labels\img_NNNN.txt     6-field PROPOSALS
        │  convert_proposals.py           (all boxes kept, conf stripped)
        ▼
<capture folder>\img_NNNN.txt             5-field, BESIDE each frame
        │  human review                   (class edits + phantom deletions)
        ▼
final real-frame test set                 scored through the letterbox
```

## Step 1 — pre-label with the metrology model

```powershell
uv run yolo detect predict model=D:/git/cart-pole/training/runs/cp-1440-base/weights/best.pt source=<capture folder> save_txt save_conf save=True imgsz=1440 project=D:/git/cart-pole/training/runs name=real-preview
```

`save=True` also writes annotated JPGs (boxes drawn) into the same
run folder — open those to see what the model saw. Proposals land in
`runs\real-preview\labels\` as 6-field files (class x y w h **conf**),
one line per *detection* — duplicates included.

## Step 2 — convert to corpus format

```powershell
uv run python src\convert_proposals.py
```

Writes `img_NNNN.txt` beside each frame in the capture folder, 5
fields, **every found box kept** (no deduplication) — class
corrections become pure text edits. Frames whose proposals were empty
are skipped and reported.

## Step 3 — human correction

Open each frame (or its annotated preview) and edit the txt:

- **Wrong class on a correct box**: change the leading id.
- **Phantom boxes** (duplicates of one physical marker, look-alike
  regions): delete the line.
- **Missing markers**: add a line in corpus format.

**Log the corrections — they are the measurement.** Count per class:
boxes accepted as-is, boxes moved, boxes deleted, markers added by
hand. That distribution IS the quantified sim-to-real gap; the
labeling drudgery is also the experiment.

Known corrections from the first batch (70 frames, 2026-10-16), to
expect and confirm:

- The **rail-right reference marker** (physically Right/596) is
  classified **End** in most frames — flip those to Right.
- The **cart pivot marker** (Pivot/920) goes undetected or is called
  **End** in frames where the cart sits near the rail-right marker —
  re-add or flip.
- **Duplicate Ends** are pole-tip + misclassified rail-right (+
  sometimes pivot) — keep one per physical marker.
- **Left** at the rail-left end was correct in all 70 frames.

## Step 4 — score the deployment model through the letterbox

```powershell
uv run yolo detect val model=D:/git/cart-pole/training/runs/cp-640-base/weights/best.pt data=<real-val>.yaml imgsz=640
```

The real set needs a small yaml pointing at its images/labels (same
shape as `datasets/cart-pole/cart-pole.yaml`, names in rig slot
order). Bar (adjustable): per-class AP50 >= 0.90, no class below
0.80. Every miss feeds the mining loop — a `ForcedSituationSO` per
failure mode, or a fine-tune on the corrected real frames.

## Findings this process produced on its first run

- The model detects **everything** on real frames — boxes are tight
  and confident. The sim-to-real gap manifested as **class confusion
  toward End**, not missed detections: the rail-right Right marker
  and the pivot marker drift to the class whose training distribution
  had the widest positional variance. Position shortcut learning,
  observed in the wild.
- **Left** (rail-left reference) transferred perfectly — 70/70
  frames, always correct class.
- Duplicates ("2-3 Ends") are physical look-alikes and reference
  markers being pulled into the End class — not NMS artifacts.

## Status of claims

- **Landed:** the 70-frame first batch (1440x1080), the pre-label
  predictions, the class-confusion diagnosis (verified against
  annotated frames), the converter script.
- **Pending:** human correction of the 70 label files, the real-val
  yaml, the deployment model's real-frame score, Pi timing.
