# 06 — Real-frame labeling: from raw capture to a scored test set

**Path: 6 of 7. Before this: 05-POST_TRAINING.md (a validated export).
After this: 07-CONVERT_PROPOSALS.md (reference for step 3).**

This loop turns a folder of raw frames from the physical rig into a
labeled real-frame test set. It uses two different models: the
**metrology model** (1440, tightest boxes) writes the draft labels,
and the **deployment model** (640) is the one being scored. The
instrument that proposes is not the instrument being judged.

## The contract

Real frames are labeled in the same format as the synthetic sets:
one `.txt` beside each frame, 5 fields per line (`class x_center
y_center w h`), normalized, one line per visible marker. Class ids
are the rig slot order: 0=Left, 1=Right, 2=Pivot, 3=End.

> ## Step 1 — capture the raw frames
>
> Why: the test set must be the same pixels the on-Pi inference
> consumes — a codec-processed or auto-exposed frame measures a
> different detector than the one that will run.
>
> ```powershell
> rpicam-still -o img_NNNN.jpg --width 1456 --height 1088 ...
> ```
>
> - Raw camera path only — never WebRTC (codec-processed,
>   bitrate-adapted).
> - 1456×1088 is the sensor's native output (the tool's default) — no
>   rescaling.
> - 1 ms shutter, gain/light set for exposure, AE/AWB locked: the
>   synthetic frames have fixed exposure; the real set must match
>   rather than inject a gap.

> ## Step 2 — pre-label with the metrology model
>
> Why: real frames have no geometry to project labels from, so a
> detector writes the first draft and the human corrects it. The 1440
> metrology model produces the tightest boxes, and it is not the model
> being judged — the proposal instrument must not be the scored one.
>
> ```powershell
> uv run yolo detect predict model=D:/git/cart-pole/training/runs/cp2-1440-base/weights/best.pt source=<capture folder> save_txt save_conf save=True imgsz=1440 project=D:/git/cart-pole/training/runs name=real-preview
> ```
>
> Expected: `runs\real-preview\labels\img_NNNN.txt` — 6-field proposals
> (`class x y w h conf`), one line per detection, duplicates included.
> `save=True` also writes annotated JPGs — open those to see what the
> model saw.

> ## Step 3 — convert the proposals to corpus format
>
> Why: the proposals live in predict's own output directory in a 6-field
> format no consumer reads — converting them to 5-field corpus lines on
> arrival means the capture folder becomes a normal dataset with no
> conversion step left in the pipeline.
>
> ```powershell
> uv run python src\convert_proposals.py
> ```
>
> Writes `img_NNNN.txt` beside each frame in the capture folder: 5
> fields, every found box kept (no deduplication). Frames whose
> proposals were empty are skipped and reported. See
> `07-CONVERT_PROPOSALS.md` for the full contract.

> ## Step 4 — human correction
>
> Why: real frames have no ground truth — the human is the class
> arbiter. The corrections are not drudgery to minimize; they are the
> measurement of the sim-to-real gap, so every edit is counted.
>
> Open each frame (or its annotated preview) and edit the txt:
>
> - **Wrong class on a correct box**: change the leading id.
> - **Phantom boxes** (duplicates of one physical marker): delete the
>   line.
> - **Missing markers**: add a line in corpus format.
>
> **Log the corrections — they are the measurement.** Count per class:
> boxes accepted as-is, moved, deleted, added by hand. That
> distribution is the quantified sim-to-real gap.
>
> Known corrections to expect from the first batch (70 frames,
> 2026-10-16):
>
> - The **rail-right reference marker** (physically Right/596) is
>   classified **End** in most frames — flip those to Right.
> - The **cart pivot marker** (Pivot/920) goes undetected or is called
>   **End** near the rail-right marker — re-add or flip.
> - **Duplicate Ends** are pole-tip + misclassified rail-right (+ sometimes
>   pivot) — keep one per physical marker.
> - **Left** was correct in all 70 frames.

> ## Step 5 — score the deployment model
>
> Why: synthetic val does not decide readiness — the deployment model
> must be judged on the pixels the rig actually produces, at the scale
> the control loop actually runs.
>
> ```powershell
> uv run yolo detect val model=D:/git/cart-pole/training/runs/cp2-640-base/weights/best.onnx data=<real-val>.yaml imgsz=640
> ```
>
> The real set needs a small yaml pointing at its images/labels (same
> shape as `datasets/generated/data.yaml`, names in rig slot order).
>
> **Bar (adjustable): per-class AP50 ≥ 0.90, no class below 0.80.**

## What the findings mean

First batch (70 frames, 2026-10-16): the model detected everything —
the sim-to-real gap manifested as **class confusion toward End**, not
missed detections. Position shortcut learning: the old corpus's fixed
camera taught "where the box is" as a feature. The new corpus's
camera variation exists to break that shortcut — the corrections log
from this batch is the test of whether it worked.

If Right→End confusion persists after retraining, escalate to the
calibration-phase design: persistent Left/Right references calibrated
per session, and a hard gate that any detected End must sit
500 mm ± ε from Pivot (the pole-length arc). That spec is decided;
it is built only if the measurements demand it.

Every real-world miss feeds the mining loop: a hand-label for the
test set, or a synthetic-corpus change for the next batch.
