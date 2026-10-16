# The Python scripts — what they do

Two scripts, both stdlib-only, both run through uv from
`D:\git\cart-pole\training\`:

```powershell
uv run python src\build_dataset.py    # corpus -> train/val dataset
uv run python src\dataset_stats.py    # dataset -> statistics report
```

## Data flow

```
datasets\cart-pole\            (corpus - READ ONLY, committed, git-clean)
  pilot-flat frames+labels     (300, the val bucket)
  run_<timestamp>\ x10         (1000 each, the train bucket)
        │  build_dataset.py
        ▼
training\datasets\cart-pole\   (GENERATED - wiped on every build)
  images\{train,val}\          hardlinks, <run>_frame_NNNNNN.jpg
  labels\{train,val}\          hardlinks, <run>_frame_NNNNNN.txt
  cart-pole.yaml               class map from markers.json
        │  dataset_stats.py
        ▼
console report                 per-class + per-run + width histogram
```

## build_dataset.py

Assembles the Ultralytics dataset from the capture corpus. Corpus in,
dataset out; the corpus is never written.

- `discover_runs(corpus)` — finds every capture location: the flat
  pilot corpus first (300 frames at the corpus root), then the
  `run_<timestamp>` children in timestamp order (zero-padded names
  sort chronologically).
- `gate_run(name, run)` — the quality gate, all derivable from the
  files (no detection, no Unity):
  - **pairing** — every `frame_*.jpg` must have a `.txt` and vice
    versa (set difference both directions);
  - **parsing** — every label line: 5 fields, class in 0-3,
    coordinates in [0,1]; each violation becomes a problem message
    with file:line address; `boxes` counts only fully valid lines;
  - empty label files are legal (all four markers behind the camera)
    and only reported.
- `place(src, dst)` — **hardlinks** the file into the dataset (one
  copy of the bytes, two names); falls back to `copy2` when the
  filesystem refuses.
- `reset_output()` — wipes and recreates `images|labels x train|val`.
  The output tree is a build artifact, fully regenerable; each build
  starts clean.
- `write_yaml()` — builds `cart-pole.yaml` from the corpus's
  `markers.json`: the class order is the **rig slot order**, read
  from disk, never retyped.
- `main()` — gates the whole corpus **before** placing a single file
  (a corrupt frame aborts the build before the output tree is
  touched), refuses with exit code 1 and a full problem list, then
  places everything and writes the yaml. Exit code 0 = built.

Split rule: **by whole run**, never by frame — frames within a run
are near-duplicates (same pools, same seed pair) and mixing them
into both buckets would inflate val. The flat pilot is the odd run
out and lands in **val**; the ten bulk runs land in **train**
(10,000 / 300).

Every placed filename is prefixed with its run's identity
(`run_20261015_002542_frame_000000.jpg`) — capture numbering restarts
per run, so unprefixed names would collide across runs and each run
would silently overwrite the previous one's files. The prefix also
makes `dataset_stats.py`'s per-run report possible: the filename
carries the metadata.

## dataset_stats.py

Reads the built dataset's labels — pure text parsing, no image
decoding — and reports per bucket:

- file and box counts (train: 10,000 files / 39,585 boxes;
  val: 300 / 1,191 at build time);
- boxes per class — the rig slot order table training results get
  compared against (class 3, End, is the one to watch);
- boxes per run — free from the filename prefix
  (`txt.name.split("_frame_")[0]`);
- box **width in pixels** (normalized width x `IMG_WIDTH` = 1440):
  min / max / mean and a 4-px-bin histogram. This is the number that
  justified `imgsz=1440` — see COMMANDS.md.

## Conventions both scripts share

- Paths derive from `Path(__file__).resolve().parents[...]` — the
  scripts work no matter where they're invoked from, and no absolute
  path is hardcoded.
- The gate is **derivable-only**: it checks what the files prove
  (pairing, class range, coordinate range). The in-Unity detection
  verification that ran at capture time is intentionally not
  re-done here.
- Everything is deterministic given the corpus: same corpus in,
  same dataset out — which is why `datasets/` is gitignored (see
  `.gitignore` in this directory).
