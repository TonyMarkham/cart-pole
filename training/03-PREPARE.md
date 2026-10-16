# Preparing to train — GPU check and the ultralytics install

Training needs an NVIDIA GPU. This file covers the two preparation
steps that precede any training command (the commands themselves live
in `04-COMMANDS.md`).

## Step 1 — check the GPU

```powershell
nvidia-smi
```

- A **table** prints: you have a usable CUDA GPU. Note two numbers
  from it — the GPU **name** and its **memory** (both feed decisions
  later, e.g. whether `batch=-1` can auto-fit).
- **"not recognized"**: no NVIDIA GPU or no driver — stop here. CPU
  training at 1440 px is a weeks-scale affair; resolve the hardware
  first (borrow, buy, rent).

What the landed check showed (2026-10-15):

- **NVIDIA GeForce RTX 3090, 24576 MiB (24 GB)**, driver 616.56,
  CUDA UMD 13.4 — a training-grade card. 24 GB means the `batch=-1`
  auto-batch at 1440 px fits comfortably; the "CUDA out of memory →
  retry with batch=8" contingency is unlikely to fire.
- A few GB may already be in use by desktop apps and Unity Editors —
  irrelevant at this scale; Unity does not need to be closed.

## Step 2 — install ultralytics (the permission gate)

```powershell
uv add ultralytics
```

This is the biggest install of the project: ultralytics plus **PyTorch
with CUDA support** — a multi-GB download, several minutes. Per the
repo's standing rule it happens only on an explicit OK.

What it changes (all inside `training\`):

- `.venv\` — gains torch, CUDA libraries, ultralytics and its
  dependencies; the system Python is never touched.
- `pyproject.toml` — `dependencies` gains the ultralytics entry.
- `uv.lock` — the resolved exact versions, committed for
  reproducibility.

Verify the install actually sees the GPU:

```powershell
uv run python -c "import torch; print(torch.__version__, torch.cuda.is_available(), torch.cuda.get_device_name(0))"
```

Expected: a torch version, `True`, and the GPU name. `False` means the
CUDA build did not land — stop and resolve before training.

### The Windows torch trap (hit for real, 2026-10-15)

`uv add ultralytics` pulled `torch-2.14.1+cpu` — the **`+cpu` is the
whole story**. PyPI's Windows torch wheels are CPU-only builds; the
CUDA builds live on PyTorch's own package index, which uv does not
consult unless told to. The GPU is irrelevant to this failure —
`torch.cuda.is_available(): False` because the installed torch
contains no CUDA code at all.

The fix has two halves, and the first is the non-obvious one.

1. **Make torch a direct dependency.** `tool.uv.sources` routes only
   the project's *direct* dependencies — torch enters here
   *transitively* (ultralytics requires it), so a sources block alone
   does nothing: the lockfile keeps `pypi.org/simple`, and `uv sync`,
   `uv lock`, even `uv lock --upgrade-package torch` all politely
   preserve the CPU build (all three observed). The uv guide's own
   PyTorch examples use the same shape: torch and torchvision listed
   in `[project.dependencies]`, routed via `[tool.uv.sources]`.

```toml
[[tool.uv.index]]
name = "pytorch-cu128"
url = "https://download.pytorch.org/whl/cu128"
explicit = true

[tool.uv.sources]
torch = { index = "pytorch-cu128" }
torchvision = { index = "pytorch-cu128" }
```

`explicit = true` routes only torch/torchvision to that index; every
other dependency stays on PyPI.

2. **Declare them, then sync** — adding a direct dependency forces a
   genuine re-lock:

```powershell
uv add torch torchvision
uv sync
```

The verification one-liner must print a version ending in `+cu128` —
not `+cpu` — before any training command runs.

Landed outcome (2026-10-15): `torch 2.11.0+cu128`, `torchvision
0.26.0+cu128`, `True`, RTX 3090. Three details worth keeping:

- **The version drops** — 2.14.1 (PyPI) → 2.11.0 (cu128 index). Not a
  mistake: 2.11.0 is the newest Windows cu128 wheel for this Python;
  the accelerator indexes trail PyPI, and ultralytics is happy with
  the result.
- **The Ultralytics header is the standing tell** — every training run
  prints its torch build (`torch-2.11.0+cu128`); `+cpu` appearing in
  that header means the trap has re-fired (fresh venv, new machine).
- **uv's hardlink warning** ("Failed to hardlink files; falling back
  to full copy") fires when uv's cache and the venv sit on different
  drives. Cosmetic — a slower install, nothing else; silence it with
  `setx UV_LINK_MODE copy` if it annoys.

Driver/runtime compatibility note: the CUDA **driver** version shown
by nvidia-smi (13.4 here) only needs to be newer than the CUDA
**runtime** the torch build ships (12.8 for cu128) — it is, and the
RTX 3090 (sm_86) is supported by all cu12x builds.

(Automated alternative: uv's `pip` interface can select the index
automatically — `uv pip install torch --torch-backend=auto` queries
the installed driver and picks the matching PyTorch index. The
project-interface route above is what this repo uses: it keeps
`uv.lock` as the reproducible record.)

## Then

Training itself: see `04-COMMANDS.md` — the 1440 base run, the 640
comparison, how to read the results, and the Phase 5 export.
