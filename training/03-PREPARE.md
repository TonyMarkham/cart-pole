# 03 — Prepare the training environment

**Path: 3 of 7. Before this: 02-SCRIPTS.md. After this:
04-COMMANDS.md. Do once.**

Two checks before any training command: the GPU is usable, and the
uv environment contains CUDA-enabled torch.

> ## Step 1 — check the GPU
>
> Why: training is CUDA-only in practice, and the GPU's memory decides
> the auto-batch and whether the chosen resolutions are affordable.
> Discovering a missing driver after a failed training night wastes the
> night.
>
> ```powershell
> nvidia-smi
> ```
>
> - A table prints: usable. Note the GPU name and memory — both feed
>   later decisions (auto-batch, resolution).
> - `not recognized`: no NVIDIA GPU or no driver — resolve the hardware
>   first. CPU training at 1440 px is a weeks-scale affair.
>
> Landed check (2026-10-15): NVIDIA GeForce RTX 3090, 24576 MiB,
> driver 616.56. 24 GB means `batch=-1` auto-fits without manual
> lowering.

> ## Step 2 — install ultralytics with CUDA torch
>
> Why: ultralytics is the training framework; torch must be the CUDA
> build or every training command runs on CPU (weeks instead of hours).
> The lockfile pins the exact versions so the environment is
> reproducible.
>
> For a new machine or fresh clone, doc 02's one-time setup (`uv sync`)
> already installs everything `uv.lock` pins — CUDA torch and
> ultralytics included. This step is only needed when re-adding
> packages or re-locking from scratch.
>
> The uv environment lives in `training\` — run everything from there.
>
> ```powershell
> cd D:\git\cart-pole\training
> uv add ultralytics
> uv add torch torchvision
> uv sync
> ```
>
> Torch must come from PyTorch's own index, not PyPI. If
> `pyproject.toml` does not yet contain this block, add it:
>
> ```toml
> [[tool.uv.index]]
> name = "pytorch-cu128"
> url = "https://download.pytorch.org/whl/cu128"
> explicit = true
>
> [tool.uv.sources]
> torch = { index = "pytorch-cu128" }
> torchvision = { index = "pytorch-cu128" }
> ```

> ## Step 3 — verify CUDA is real
>
> Why: the `+cpu` trap fails silently — a CPU torch installs cleanly,
> imports cleanly, and only reveals itself as a 100× slowdown (or an
> out-of-memory error) mid-training. Thirty seconds here saves a night.
>
> ```powershell
> uv run python -c "import torch; print(torch.__version__, torch.cuda.is_available(), torch.cuda.get_device_name(0))"
> ```
>
> Expected: a version ending in `+cu128`, `True`, and the GPU name.

## Troubleshooting

- **`+cpu` in the version**: the CPU-only build installed. PyPI's
  Windows torch wheels are CPU-only; the CUDA builds live on the
  PyTorch index. The fix is the two-part config in step 2: torch as a
  *direct* dependency plus the `[tool.uv.sources]` routing. A sources
  block alone does nothing — transitive dependencies keep the PyPI
  index. Then `uv add torch torchvision` and `uv sync` again.
- **Version drops after the fix** (e.g. 2.14.1 → 2.11.0): expected.
  The accelerator indexes trail PyPI; ultralytics is fine with it.
- **uv hardlink warning**: cosmetic — cache and venv on different
  drives. `setx UV_LINK_MODE=copy` silences it.
- **Standing tell**: every training run prints its torch build in the
  header. `+cpu` there means this trap re-fired (fresh venv, new
  machine).
