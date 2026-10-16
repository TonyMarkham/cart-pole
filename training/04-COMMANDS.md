# Training commands — what they do, what to expect

Every command runs from `D:\git\cart-pole\training\` through the uv
environment (`uv run ...`). Nothing here touches the corpus; training
reads the built dataset under `datasets/cart-pole/`.

## The base run (native 1440)

```powershell
uv run yolo detect train model=yolo11n.pt data=datasets/cart-pole/cart-pole.yaml imgsz=1440 batch=-1 epochs=200 patience=50 device=0 project=D:/git/cart-pole/training/runs name=cp-1440-base
```

Knob by knob:

| Knob | Value | Meaning |
|---|---|---|
| `model` | `yolo11n.pt` | Smallest YOLO11, starting from public pretrained weights. The file is **fetched automatically on first use** (~5 MB) — a one-time, expected pause. |
| `data` | the yaml | Written by `build_dataset.py` from the run's `markers.json`; the class order inside is the rig slot order (0=Left, 1=Right, 2=Pivot, 3=End). |
| `imgsz` | 1440 | Native camera scale. The dataset's own box-width histogram justified this: boxes are ~22.5–46 px wide at 1440, but only ~10–20 px at 640 — below the detector's stride-8 comfort zone. |
| `batch` | -1 | Auto-fit the batch size to GPU memory (RTX 3090, 24 GB — should not need manual lowering). |
| `epochs` | 200 | Upper bound on passes over the dataset. |
| `patience` | 50 | Early stop when val mAP stops improving for 50 epochs. Early stopping is normal and good. |
| `device` | 0 | The first CUDA GPU. |
| `project`/`name` | absolute `training\runs` path / `cp-1440-base` | Output directory `training\runs\cp-1440-base\`. |

First-run sequence: dataset scan → auto-batch CUDA probe → AMP check →
per-epoch progress bars. The first epoch also confirms the pipeline end
to end; the full run takes **hours** (10,000 frames at 1440 px).

### The `project=` trap (hit for real, 2026-10-15)

A **relative** `project=runs` is resolved against Ultralytics' own
settings (`AppData\Roaming\Ultralytics\settings.json`, created on
first run), **not** the working directory — the aborted first attempt
landed at `D:\git\cart-pole\runs\detect\runs\cp-1440-base`: outside
`training\`, gitignored by nothing, with a doubled `detect\runs\`
segment. **Always pass `project=` as an absolute path.** Related
strays from the same session: the AMP check downloads `yolo26n.pt`
into a `weights\` folder relative to that same root (safe to delete;
it re-downloads as needed).

AutoBatch note: it chose **batch 13** at 58% GPU memory utilization —
Ultralytics is conservative by default (60% fraction). That is fine;
do not chase the remaining VRAM.

## The comparison run (deployment scale, 640)

Same data, one knob changed — this answers once whether the Pi 5 can
afford native-scale inference or must upscale:

```powershell
uv run yolo detect train model=yolo11n.pt data=datasets/cart-pole/cart-pole.yaml imgsz=640 batch=-1 epochs=200 patience=50 device=0 project=D:/git/cart-pole/training/runs name=cp-640-base
```

If the small-end boxes hurt at 640, also try the small-object variant
`yolo11n-p2.pt` (adds the P2/stride-4 head). At 1440 the standard head
stack is fine. Compare the two runs' **per-class** final mAP50-95 from
their `results.csv` — same split, so the comparison is fair by
construction.

## Reading the results

All output lands in `training\runs\<name>\`:

- `results.csv` — one row per epoch; the last row is the summary. The
  columns with `mAP50-95` are the accuracy scores.
- `weights\best.pt` — the checkpoint to keep and export.
- `weights\last.pt` — the final-epoch checkpoint (not the best one).
- `confusion_matrix.png` — rows/columns are the four classes plus
  background; a bright off-diagonal cell names the confusion.

Read the four classes **separately** — an aggregate mAP can hide a
failing End marker. Pick checkpoints by val scores, never by training
loss.

## Training policy (what was deliberately left alone)

- Frames are trained on **as written** — q90 JPEG statistics are part
  of the domain; no re-encoding, no PNG copies.
- Augmentation stays at Ultralytics defaults, which already match this
  domain: `fliplr=0.5` on (the rig is near-symmetric), `degrees=0`,
  `perspective=0` (the physical camera never rolls or warps),
  `copy_paste=0` (the corpus carries boxes, not segmentation masks).

## Export (Phase 5)

```powershell
uv run yolo export model=D:/git/cart-pole/training/runs/cp-1440-base/weights/best.pt format=onnx imgsz=1440
```

Produces `best.onnx`. `half=False` (default) is correct for the Pi 5's
CPU runtime. The Pi-side inference must letterbox preprocessing exactly
like Ultralytics (scale to fit, pad) — validate the exported model
against corpus frames in the uv env before it touches the Pi.
