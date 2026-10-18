# Training commands — the walkthrough, step by step

**Path: 4 of 7 — train the two models and prove them. Steps 1–3
executed (2026-10-17); landed results recorded in place.** Purpose:
produce the metrology model (1440) and the deployment model (640),
and prove the deployment scale is right.

**Every command: `cd D:\git\cart-pole\training` first.** `uv run`
resolves the Python environment from the working directory's
`pyproject.toml` — from any other directory it fails or trains in the
wrong environment (a `+cpu` torch with no CUDA). `data=` and
`project=` are absolute, so nothing else depends on the cwd.

The corpus is `datasets/generated/` (11,000 greyscale frames — see
`01-CAPTURE.md`); the yaml was written by `build_dataset.py`
(`02-SCRIPTS.md`). Nothing in this file touches the corpus.

> ## Preamble — what training does, and the yaml that runs it
>
> Training is a loop, and `datasets/generated/data.yaml` is its only
> instruction sheet. The yaml (written by `build_dataset.py`,
> 02-SCRIPTS.md) names the two buckets: `train:` — Set-01…Set-10,
> 10,000 frames the weights learn from — and `val:` — Set-11's 1,000
> frames the weights never learn from, plus the class names (Left,
> Right, Pivot, End) that give every score table its rows.
>
> Each epoch is one pass through the training frames, followed by a
> scoring pass over the val frames — the same per-class table the
> steps below read. The val scores never touch the weights; they do
> two jobs instead: `best.pt` is the snapshot that produced the best
> val table (which is why "best at epoch 324" can differ from the
> stop at epoch 374), and `patience=50` ends training after 50
> epochs without a val improvement.
>
> The same yaml is reused downstream, unchanged: the export check
> (05, Step 2) scores the `.onnx` against the same `val:` set and the
> same names. One split, written once — every tool in the pipeline
> agrees on what "held out" means; nothing re-decides it.
>
> Because Set-11 doubles as the per-epoch judge, a score on it is
> slightly optimistic: the checkpoint was partly chosen for doing
> well on exactly those frames. The final arbiter is the real-rig
> set (06-REAL_LABELING.md).

> ## Step 1 — train the 1440 metrology model
>
> Why: 1440 is the corpus's native scale, where boxes are largest and
> boxes tightest — this model is the measuring stick (real-frame
> proposals, audits) and the accuracy ceiling the 640 deployment model
> is measured against.
>
> ```powershell
> uv run yolo detect train model=yolo11n.pt data=D:/git/cart-pole/datasets/generated/data.yaml imgsz=1440 batch=-1 epochs=500 patience=50 device=0 project=D:/git/cart-pole/training/runs name=cp2-1440-base
> ```
>
> - `imgsz=1440` — the corpus's box widths (20.5–45.1 px at 1456) need
>   the resolution; 1456 is not a multiple of 32, so Ultralytics
>   letterboxes 1456→1440 (a 1% scale change).
> - `epochs=500` — `patience=50` does the actual stopping.
>
> **Landed (2026-10-17):** early-stopped at epoch 374, best at 324,
> 14.9 h, batch 15. Per-class mAP50-95: all 0.982, Left 0.989,
> Right 0.955, Pivot 0.993, End 0.990. 2.2 ms inference.

> ## Step 2 — check: does the 1440 model survive 640 inference? (free, minutes)
>
> Why: it answers whether a 640 deployment model is worth training at
> all — for free, in minutes. If the tightness tax were a class
> failure rather than uniform, the whole 640 plan would change before
> the 640 training spends a night.
>
> ```powershell
> uv run yolo detect val model=D:/git/cart-pole/training/runs/cp2-1440-base/weights/best.pt data=D:/git/cart-pole/datasets/generated/data.yaml imgsz=640 project=D:/git/cart-pole/training/runs name=cp2-1440-crossres-640
> ```
>
> **Landed:** mAP50 held (0.992) but mAP50-95 fell to 0.872 with a
> near-uniform per-class tax — pure resolution cost. That gap is what
> step 3 closes.

> ## Step 3 — train the 640 deployment model
>
> Why: the Pi's CPU can only afford 640-scale inference, and training
> at the inference scale recovers most of the letterbox tax (the old
> corpus: 95%). This produces the model the control loop actually runs.
>
> ```powershell
> uv run yolo detect train model=yolo11n.pt data=D:/git/cart-pole/datasets/generated/data.yaml imgsz=640 batch=-1 epochs=500 patience=50 device=0 project=D:/git/cart-pole/training/runs name=cp2-640-base
> ```
>
> **Landed (2026-10-17):** all 500 epochs, 4.5 h, batch 86. Per-class
> mAP50-95: all 0.971, Left 0.971, Right 0.949, Pivot 0.983, End 0.982.
> 0.6 ms inference — 90% of the step-2 gap recovered for 3.7× less
> compute than step 1. **This is the deployment model.**
>
> If small boxes hurt at 640, the small-object variant `yolo11n-p2.pt`
> (adds the P2/stride-4 head) is the follow-up experiment. At 1440 the
> standard head stack is fine.

## Reading the results

All output lands in `training\runs\<name>\`:

- `results.csv` — one row per epoch; the last row is the summary.
- `weights\best.pt` — the checkpoint to keep (val-selected).
- `weights\last.pt` — the final-epoch checkpoint (not the best one).
- `confusion_matrix.png` — bright off-diagonal cells name confusions.

Read the four classes **separately** — an aggregate mAP can hide a
failing marker. Pick checkpoints by val scores, never by training
loss.

## Gotchas (all hit for real)

- **`project=` must be absolute.** A relative `project=runs` resolves
  against Ultralytics' own settings dir — the aborted first attempt
  landed at `D:\git\cart-pole\runs\detect\runs\cp-1440-base`: outside
  `training\`, gitignored by nothing.
- **Do not chase VRAM.** AutoBatch targets 60% utilization on purpose
  (chose 15 at 1440, 86 at 640).
- **Frames are trained on as written** — q90 JPEG statistics are part
  of the domain; no re-encoding, no PNG copies. Augmentation stays at
  Ultralytics defaults (`fliplr=0.5`; `degrees=0`, `perspective=0`,
  `copy_paste=0`).
- The old-corpus results (`cp-1440-base`, `cp-640-base`) are retired
  records in `05-POST_TRAINING.md` — compare, don't continue.
