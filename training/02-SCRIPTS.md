# 02 — Gate the corpus and write the yaml

**Path: 2 of 7. Before this: 01-CAPTURE.md. After this: 03-PREPARE.md
(environment) and 04-COMMANDS.md (training).**

`build_dataset.py` checks every frame/label pair in the corpus and
writes `datasets/generated/data.yaml` — the file Ultralytics trains
from. Nothing is copied: the yaml points at the run folders and
Ultralytics reads them in place.

> ## Step 1 — one-time setup (new machine or fresh clone only)
>
> Why: `uv run` requires the project's `.venv`, which does not exist on
> a fresh clone — and the scripts need the exact locked package
> versions, or the training environment will not match these docs.
>
> 1. Install uv itself:
>
>    ```powershell
>    powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
>    ```
>
> 2. Initialize the environment from the committed lockfile:
>
>    ```powershell
>    cd D:\git\cart-pole\training
>    uv sync
>    ```
>
>    `uv sync` creates `.venv\` and installs exactly what `uv.lock`
>    pins — including CUDA torch and ultralytics. First sync downloads
>    several GB; that is expected. If the resulting torch is `+cpu`
>    instead of `+cu128`, see 03-PREPARE.md's troubleshooting.

> ## Step 2 — run the gate
>
> Why: a training run costs hours — this gate catches broken or
> mislabeled data in seconds, before that cost is spent. It also writes
> `datasets/generated/data.yaml`, the file Ultralytics requires to find
> the data at all.
>
> ```powershell
> uv run python src\build_dataset.py
> ```
>
> (`uv run` resolves the environment from the working directory's
> `pyproject.toml` — run it from `training\`.)

> ## Step 3 — read the output
>
> Why: confirm the counts match what doc 01 recorded — a mismatch means
> a run folder is partial or missing before training starts.
>
> Expected:
>
> ```
> train: 10 runs, 10000 frames
> val:   1 runs, 1000 frames
> dataset: 11000 frames, 42769 boxes
> yaml: D:\git\cart-pole\datasets\generated\data.yaml
> ```
>
> - Empty label files are legal (no markers confirmed in frame) and are
>   only reported.
> - The gate is derivable-only: pairing (every image has a label and
>   vice versa), 5-field grammar, class ids 0–3, coordinates in [0, 1].
>   The detection-level verification already ran at capture time.

## If the gate refuses

`BUILD REFUSED - N problem(s)` with a file:line list means the corpus
is damaged. Fix or delete the named files, then rerun. Do not train
on a refused tree — the check is cheaper than the hours a broken
dataset wastes.

## Split rule

Whole runs only — frames within a run are near-duplicates and mixing
a run across buckets inflates val. Default: the newest run is val,
the rest train. Override:

```powershell
uv run python src\build_dataset.py --val Set-05
```

## The yaml it writes

```yaml
train:
- run_Set-01_...
val:
- run_Set-11_...
names:
- Left
- Right
- Pivot
- End
channels: 1
```

`names:` is read from the runs' `markers.json` (rig slot order) — it
is never retyped. `path:` is omitted; it defaults to the yaml's own
directory.
