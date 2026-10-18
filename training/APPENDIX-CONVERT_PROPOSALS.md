# 07 — convert_proposals.py, explained

**Appendix — not on the path. Reference for 06-REAL_LABELING.md step
5.** Purpose:
the full contract of the proposal converter — what it reads, what it
writes, what it keeps, and what it never touches — so the human
refinement ladder (07-REFINING.md, Rung 1) starts from a known,
trustworthy state.

## Why this exists

Real frames have no labels and no ground truth. On synthetic frames
the labels are computed exactly from the 3D model — on real frames
there is nothing to project, so a first draft of the boxes must come
from somewhere. The detector writes that draft: it "proposes" where
the markers are, and the human then corrects the proposal into truth.

Ultralytics' predict saves those drafts in its own folder and its own
format — 6 fields per line, with an extra confidence column. The
trainer and the scoring tools read only the corpus format: one 5-field
`.txt` per frame in the run's `labels\` folder. The converter is the
bridge between those two shapes, for **both** pre-named trees: the
1440's drafts become the scoring set the human corrects, and the
640's drafts become the deployment-path proof record. Without it, the
real frames can never be scored or proofed — and that is the entire
sim-to-real experiment.

## How to run it

```powershell
cd D:\git\cart-pole\training
uv run python src\convert_proposals.py
```

No arguments — the datasets are fixed constants in the file, matching
the pre-named doc 06 datasets. They never change. One run converts
both:

| dataset | reads (6-field proposals) | writes (5-field labels) |
|---|---|---|
| `real-1440` | `training\runs\1440\labels\` | `datasets\real-1440\data\labels\` |
| `real-640` | `training\runs\640\labels\` | `datasets\real-640\data\labels\` |

One 5-field `.txt` per frame in the dataset's `data\images\` folder —
the confidence column stripped, byte-for-byte the synthetic label
format.

## What it keeps

**Every box the detector found, unchanged in position and size.**
Nothing is deduplicated, nothing is reclassified, nothing is deleted.
If a frame has three "End" boxes, the label file has three lines.

Why: the label refinement (07-REFINING.md, Rung 1) is a pure text edit —
change a leading class id, or delete a line that shouldn't exist.
That only works if the converter made every decision reversible and
nothing was silently discarded.

## What it never touches

The frames — never opened for writing. The proposal files — read-only,
left in place. Everything outside the two datasets — untouched. The
only files it creates are the label files, and each dataset's
`labels\` folder if missing.

## Self-diagnosing, per dataset

A dataset is **skipped with a message naming the missing step** when a
prerequisite is absent:

- no `real-<n>\data\images\` → "land frames first (doc 06, Step 2)";
- no `training\runs\<n>\labels\` → "run the `<n>` predict first
  (doc 06)" — the 1440 message points at Step 3, the 640 message at
  Step 4.

The converter exits with an error only when *neither* dataset could be
converted. A frame whose proposal file is missing or empty still gets
a label file — an **empty** one. That is legal corpus state (no
markers proposed), and the human pass decides what it means.

## What you see when you run it

One console line per frame, tagged with its dataset:

```
real-1440 img_3.txt: 4 boxes, classes ['0', '1', '2', '3']
real-640 img_3.txt: 4 boxes, classes ['0', '1', '2', '3']
```

— box count and which class ids are present, so you know what to
expect before opening the file. Then a per-dataset summary:

```
real-1440: 147 label files written, 0 empty, to D:\git\cart-pole\datasets\real-1440\data\labels
real-640: 147 label files written, 0 empty, to D:\git\cart-pole\datasets\real-640\data\labels
```

## After it runs

`real-1440\data\labels\` holds the metrology view — the class ids
contain the 1440's inferences; evaluate them in the viewer
(06-REAL_LABELING.md step 6), and refine via 07-REFINING.md if the
evaluation shows systematic divergence. `real-640\data\labels\`
holds the deployment-path record — the same frames as the Pi's model
generates them, proofed in the same evaluation.
