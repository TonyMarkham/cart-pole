# 07 — convert_proposals.py, explained

**Path: 7 of 7. Reference for 06-REAL_LABELING.md step 3.** Purpose:
the full contract of the proposal converter — what it reads, what it
writes, what it keeps, and what it never touches — so the human
correction step (06, step 4) starts from a known, trustworthy state.

## Why this exists

Real frames have no labels and no ground truth. On synthetic frames
the labels are computed exactly from the 3D model — on real frames
there is nothing to project, so a first draft of the boxes must come
from somewhere. The detector writes that draft: it "proposes" where
the markers are, and the human then corrects the proposal into truth.

Ultralytics' predict saves those drafts in its own folder and its own
format — 6 fields per line, with an extra confidence column. The
trainer and the scoring tools read only the corpus format: one 5-field
`.txt` beside each frame. The converter is the bridge between those
two shapes, so the real frames become a normal dataset your
corrections can turn into the test set. Without it, the real frames
can never be scored — and scoring them is the entire sim-to-real
experiment.

## How to run it

```powershell
cd D:\git\cart-pole\training
uv run python src\convert_proposals.py
```

No arguments — the two directories are constants at the top of the
file.

## What it reads

`D:\git\cart-pole\training\runs\real-preview\labels\img_NNNN.txt` —
the metrology model's 6-field proposals (`class x_center y_center
width height confidence`), one line per detection. These live in
predict's own output directory, in a format no consumer reads.

## What it writes

One 5-field `.txt` beside each frame in the capture folder:
`class x_center y_center width height` — the confidence column
stripped. Byte-for-byte the same format as the synthetic labels.

## What it keeps

**Every box the detector found, unchanged in position and size.**
Nothing is deduplicated, nothing is reclassified, nothing is deleted.
If a frame has three "End" boxes, the label file has three lines.

Why: the human correction step (06, step 4) is a pure text edit —
change a leading class id, or delete a line that shouldn't exist.
That only works if the converter made every decision reversible and
nothing was silently discarded.

## What it never touches

The frames — never opened for writing. The proposal files — read-only,
left in place. The corpus, the runs, anything else on any drive. The
only files it creates are the new `.txt` files, none of which exist
until this script runs, so nothing can be overwritten.

## What you see when you run it

One console line per frame:

```
img_0000.txt: 4 boxes, classes ['0', '2', '3']
```

— box count and which class ids are present, so you know what to
expect before opening the file. Then a final count: `70 label files
written`.

## After it runs

The capture folder is a normal dataset in corpus format. The class
ids contain the detector's known mistakes — fix the id or delete the
line (06-REAL_LABELING.md step 4), then score the deployment model
(06, step 5).
