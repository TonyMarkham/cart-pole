# The dataset format

One page that defines what a "dataset" is at every stage of the
pipeline. There is exactly **one format** — Ultralytics' default
dataset layout — and there are **no exceptions**: every use case below
writes it, reads it, or converts into it on arrival. The repo and both
dataset trees hold data in no other shape; nothing consumed as data
exists outside the structure. This file is a design doc: once
implemented, it dissolves into the numbered docs (01, 03, 04, 06, 07)
and is deleted.

## The principle: one format, everywhere

Every producer writes it and every consumer reads it — the Unity
capture, the Pi real capture, verification, the label editor, humans,
and the trainer. There is no Preview contract, no Training contract,
no staging shape, and no conversion boundary between stages, because
the format **is** the layout Ultralytics scans natively: `images/` and
`labels/` sibling folders, paired by stem, with a `data.yaml` at the
tree root. Label discovery in Ultralytics is a path substitution (last
`/images/` component → `/labels/`, extension → `.txt`), which is
exactly the layout below — consistency with the tool default is why
there is nothing else to define.

Out of scope entirely: the StateServer/gym bridge's render traffic.
It is the control loop's plumbing, carries no dataset data, and
appears nowhere below.

## The format

```
datasets\cart-pole\
  data.yaml                 path:, train:, val:, names: - the default keys
  run_<timestamp>/
    markers.json            the class map (root)
    images/
      frame_000000.jpg      1456x1088 JPEG q90, greyscale
      ...
    labels/
      frame_000000.txt      its label, 5 fields per line
      ...
```

- **The yaml:** a standard `data.yaml` at the tree root, exactly the
  default keys — `train:` and `val:` list run folders (whole runs,
  never frames; the scan is recursive, so the run folder alone
  suffices), `names:` are written from the runs' `markers.json`,
  `channels: 1` carries greyscale. `path:` may be omitted: it
  defaults to the yaml's own directory, so the run entries are
  written relative to it. Nothing in the repo converts or copies
  dataset bytes.
- **Trainer sidecars:** the first training or val pass decorates the
  tree with Ultralytics' own scan caches. Per split, one
  `labels.cache` lands beside the `labels/` folder of whichever run
  sorts first (its hash is the file paths, sizes and mtimes of the
  whole split — the cache path is the first label's parent directory,
  suffixed). With `cache='disk'`, an `.npy` is also written beside
  each image. Both are the trainer's regenerable artifacts: repo
  tools never write, read or clean them, they carry no dataset
  semantics, and deleting one only forces a rescan.
- **Pairing:** stems match across `images/` and `labels/`. That
  pairing is the format's whole mechanism, and it is Ultralytics' own
  — no manifest, no index, no sidecar list.
- **Image spec:** 1456x1088 JPEG, quality 90, **greyscale**. 1456x1088
  is the IMX296's native sensor output — the capture tool's own
  default, and the only full-size read of the sensor without rescaling
  it (the driver's smaller mode is a crop, not a scale). Greyscale is
  the decided training domain (the sensor itself is colour —
  IMX296LQR); the yaml carries `channels: 1`. The q90 contract: train
  and inference must share the same lossy statistics. Deployment
  inference runs at **728x544** (exact integer half) for small-model
  speed — inference input, not a collected shape; the capture spec
  above is the only one that exists.
- **Label grammar:** `class x_center y_center w h` — integers and
  floats, all four coordinates normalized to [0, 1], top-left origin
  (y is flipped from Unity's bottom-up viewport). One line per visible
  marker. The trainer's own scan is slightly looser than the grammar:
  it accepts coordinates up to 1.01 (a 1% tolerance) and **silently
  drops duplicate rows** — which is why one line per physical marker
  is a rule of this format, and why the editor's dedupe operation
  exists rather than being left to the trainer.
- **Empty label files are legitimate** — a frame where every marker's
  quad was behind the camera writes an empty txt. That is information,
  not corruption; the trainer counts it as a background frame (as it
  would a missing txt — but the format's writers always pair the txt,
  so absence is never meaningful here).
- **Class map:** ids are the **rig slot order** — 0=Left (tag 376),
  1=Right (596), 2=Pivot (920), 3=End (937). `markers.json` is the
  machine-readable source of that map; every consumer — human, editor,
  yaml writer — reads it from disk and never retypes it.
- **Never-wipe rule:** a writer updates a label only with *confirmed*
  lines and refuses to rewrite when confirmations are zero. No code
  path may empty a label file. (Inherited from the capture verify
  step; every writer of `labels/` carries the same rule.)
- **`wip~` — the run's instrument output.** A dataset folder may carry
  a `wip~` child holding the instruments that produce or check the
  format, never a second shape of it: the synth capture's 8K renders
  and per-marker verification crops (the never-wipe gate's evidence),
  per-frame state manifests, `verification_stats.json`, and the
  editor's corrections log. Unity- and git-ignored; no consumer of the
  format reads it. The trees contain no data outside the structure
  above plus these instruments.

## Every data use case

The format section is the contract; this table is its complete
application. Every producer and consumer of dataset data, with exactly
what it writes and reads. No use case outside this table touches
dataset data.

| Use case | Writes | Reads |
|---|---|---|
| Unity synth capture (`RigStateCapture`) | the run folder — `images/`, `labels/`, `markers.json` — plus its `wip~` instruments | the rig's Marker SOs (source of the class map) |
| Capture verification | `verification_stats.json` in `wip~` only | nothing from disk — it detects the 8K crops it just rendered, in memory; the result gates the label write (never-wipe) |
| Capture report window | nothing | `markers.json`, `labels/`, the run's verification stats |
| Tree yaml writer (training harness) | the tree's `data.yaml` | the listed runs' `markers.json` |
| Trainer, train/val | Ultralytics' own caches only — `labels.cache` per split, `.npy` beside each image with `cache='disk'` | the tree, only through `data.yaml` |
| `yolo predict` (metrology proposals, previews) | its proposals in its own output directory — never inside a dataset tree | the `images/` it is pointed at |
| Pi real capture (`rpicam-still`) | raw frames, device-resident | nothing |
| Label editor, import | the target dataset folder's `labels/` | the predict proposals, exactly once |
| Label editor, review | `labels/` + the corrections log in `wip~` | `images/`, `labels/`, `markers.json` |
| Deployment inference | no dataset artifacts — 728x544 frames are inference input | the trained model |
| Humans | nothing by hand — every human edit goes through the editor | everything |

## The two capture producers

- **(a) Synth gen** — the Unity capture writes one dataset folder per
  run: `run_<timestamp>/` under `datasets\cart-pole\`, with `images/`,
  `labels/`, and `markers.json`. Labels are projected from marker
  quads with the camera in its render configuration, gated by the
  in-memory 8K crop verification (the console's final
  `verification stats` line; a training batch must reach it). Runs can
  never mix and no directory preparation exists.
- **(d) Pi real gen** — same layout, own tree: real dataset folders
  live under `datasets\cart-pole-real\`, laid out identically with
  their own `markers.json` and their own `data.yaml`. The real sets
  are *evaluation* data: a separate tree means the synthetic tree's
  `data.yaml` can never list them — the split is enforced by path,
  not by discipline. Frames are named `frame_NNNNNN` at import, so
  every tool downstream works on real and synthetic folders
  identically.

Real-frame flow (the only place a detector proposes labels):

```
raw frames (rpicam-still, 1456x1088 native = the tool default,
            1 ms shutter, gain/light set for exposure, AE/AWB locked)
        |  yolo predict (metrology model, imgsz=1456, save_txt save_conf)
        v
proposals (predict's own output dir)   <- converted on arrival
        |  the label editor's import
        v
dataset folder (labels/ written, all boxes kept)
        |  human review (class edits + phantom deletions + re-adds)
        v
real dataset folder -> the real tree's data.yaml -> score
```

The proposal instrument is the **metrology model** — trained on the
corpus, selected for the tightest boxes; the model being *judged* is
the deployment model, never the proposal instrument. **Corrections are
the measurement**: per class, count boxes accepted as-is, moved,
deleted, added by hand — that distribution is the quantified
sim-to-real gap.

Raw Pi frames are device-resident until import; the import is the
boundary that renames them to `frame_NNNNNN`, places them in the
dataset folder's `images/`, and fills `labels/` with the imported
proposals. After import no raw frame is referenced again — the repo
holds frames in the format's shape only. Predict's 6-field output
lives and dies inside predict's own output directory: the editor's
import converts it into 5-field `labels/` lines on arrival, and no
consumer reads the 6-field form.

## The trainer (Ultralytics)

- **train / val consume the tree through its `data.yaml`** — the
  default keys above; `train:` and `val:` list **whole run folders**,
  never frames (frames within a run are near-duplicates; mixing a run
  across buckets inflates val). `names:` are written from the runs'
  `markers.json`, never retyped.
- Trained weights land in Ultralytics' own `runs/` output — model
  artifacts, not dataset data, and never inside a dataset tree.
- Val on a real-frame set uses the real tree's own `data.yaml`
  (`datasets\cart-pole-real\`, same shape, `names:` in rig slot order)
  — never a hand-edited entry in the synthetic yaml. **`train:` is a
  required key in every data.yaml** — the loader refuses a yaml
  without it (`check_det_dataset` raises before any split is read) —
  so the real yaml lists its own run folders under `train:` as well;
  the key is present for validity and simply unused for training. The
  path-enforced split stands: the synthetic yaml lists only synthetic
  runs, the real yaml only real runs.

## Stage (e) — the label editor

The editor exists because the human correction step is the pipeline's
bottleneck and its measurement both: done in a text editor, class
corrections are error-prone and unlogged. One tool, speaking the
format natively, serving both the synthetic verify path and the
real-frame path.

**Reads/writes:** a dataset folder in place — `images/` + `labels/` +
`markers.json` for class names and colors (never hardcoded). No
database, no project files; the dataset folder *is* the store.

**Operations per frame:**

- Draw the frame with its boxes, class-colored from `markers.json`.
- **Reassign class** per box (the primary real-frame operation: rail-
  right Right mislabeled End, pivot Pivot mislabeled End).
- **Delete** phantom/duplicate lines (duplicates = multiple lines for
  one physical marker — keep one per marker).
- **Add** a missing marker by drawing a box (undetected pivot).
- **Zoom to the letterbox view** — overlay the letterbox the
  deployment model actually sees (its imgsz applied to the 728x544
  inference frame), so review happens on what the detector consumes,
  not on the full frame alone.

**Speed contract:** keyboard-first. Class keys 0–3, accept/reject,
next/previous frame, all one-handed. The correction drudgery is the
sim-to-real experiment; the editor's job is to make the experiment
cheap, not to hide it.

**Accounting contract:** every edit session writes a corrections log
(per frame, per class: accepted / moved / deleted / added) into the
dataset folder's `wip~`. This is the record the text-editing workflow
asks the human to keep by hand — the editor makes it free, and it
feeds the failure-mode mining loop directly.

**Safety contract:** the never-wipe rule, inherited. The editor writes
only the lines present after review, refuses to save a wholesale
emptying of a label file, and never renames or moves frames.

**Import:** accepts predict's proposals and converts them into
5-field lines in `labels/` on arrival, all boxes kept.
