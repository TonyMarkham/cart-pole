# train — teaching the detector to find the markers

The robot balances the pole by watching four printed markers (ArUco tags) on
the rig through its camera. A **detector** is a small neural network that takes
one camera frame and returns a box around each marker it sees, with the
marker's class: idler, motor, pivot or tip. This folder trains that detector,
a YOLO model from Ultralytics, on frames that Unity rendered and labelled
by itself, so no one had to label them by hand.

It reads `../datasets/generated/` (Unity's output, one `run_Set-NN_<timestamp>/`
folder per generation set) and writes trained models to `runs/`.

All the code here is new (2026-11-10). It replaces the repo root's
`training/` folder and borrows nothing from it.

## What is in this folder

```text
__object_detection__/
├── datasets/generated/   the frames and labels Unity generated (not in git)
└── train/                this folder
    ├── pyproject.toml    the Python packages this project needs
    ├── uv.lock           the exact versions uv installed, so a rebuild matches
    ├── .venv/            the installed packages (not in git, made by uv sync)
    └── README.md         this file
```

**uv** is the tool that installs Python and the packages a project needs. It
keeps them in `.venv/` inside the project, apart from every other Python on
the computer. A **uv project** is a folder that has a `pyproject.toml`.

> ## Step 1 — set up the Python environment (done 2026-11-10)
>
> Run this once from this folder. Run it again whenever `pyproject.toml` changes:
> 
> > [!IMPORTANT]
> > Run from `__object_detection__/train/`
>
>> ### PowerShell 
>> ```pwsh
>> uv sync
>> ```
>
> This reads `pyproject.toml`, works out a set of versions that fit together,
> writes them to `uv.lock`, and installs them into `.venv/`. On 2026-11-10 it
> installed:
>
> | Package | Version | Why it is here |
> |---|---|---|
> | Python | 3.12.14 | `pyproject.toml` asks for 3.12 or newer |
> | ultralytics | 8.4.210 | the YOLO models, their training loop, the export to ONNX |
> | torch | 2.11.0+cu128 | the maths library Ultralytics trains with |
> | torchvision | 0.26.0+cu128 | torch's image tools, which Ultralytics also uses |
>
>> ### What `pyproject.toml` says
>>
>> The file is kept bare. Each part is explained here:
>>
>> - **`[project]`**: the project's name (`train`) and version. `requires-python
>>   = ">=3.12"` means any Python from 3.12 on. uv picked 3.12.14.
>> - **`dependencies`**: the three packages this project uses directly. Each
>>   `>=` is the lowest version allowed. uv installs the newest version that fits,
>>   and records it in `uv.lock`. The lowest versions are the ones the old
>>   `training/` project trained with, which are known to work on this PC:
>>   - `ultralytics`: the YOLO models, their training loop and the export to
>>     ONNX, the file format the robot's computer loads.
>>   - `torch`: PyTorch, the maths library Ultralytics trains with.
>>   - `torchvision`: torch's image tools, which Ultralytics also uses.
>>
>>   Everything else that was installed, such as numpy and OpenCV, came in
>>   because these three need it.
>> - **`[[tool.uv.index]]`**: adds a second place to download packages from,
>>   PyTorch's own index at `https://download.pytorch.org/whl/cu128`. Its torch
>> builds are for NVIDIA's CUDA 12.8, so they run on the graphics card. The
>> plain torch from PyPI (Python's main package index) runs only on the
>> processor on Windows. `explicit = true` means uv uses this index only for
>> the packages named in the next part.
>> - **`[tool.uv.sources]`**: names those packages, `torch` and `torchvision`.
>>   Everything else still comes from PyPI.
>>
>> That is what the `+cu128` at the end of torch's and torchvision's versions in
>> the table above means.
> 

> ## Step 2 — check the graphics card is used (done 2026-11-10)
>
> Training on the processor would take days instead of hours, so check that torch
> can see the card before training:
>
> > [!IMPORTANT]
> > Run from `__object_detection__/train/`
> 
>> ### PowerShell
>> ```pwsh
>> uv run python -c "import torch; print(torch.cuda.is_available(), torch.cuda.get_device_name(0))"
>> ```
>
> `uv run` runs the command inside this folder's `.venv/`. Run it from this
> folder: from anywhere else uv cannot find this project. On 2026-11-10 it
> printed:
>
>> ### Expected Results:
>> ```text
>> True NVIDIA GeForce RTX 3090
>> ```
>
> `True` means CUDA works. If it prints `False`, torch came from PyPI instead of
> PyTorch's index. Check `[tool.uv.sources]` in `pyproject.toml` and run
> `uv sync` again.

> ## Step 3 — train the robot's detector (not run yet)
>
> Training shows the model the training frames over and over, and after each
> pass checks it against frames it never learns from. Ultralytics does all of
> it: the command below is the whole training program, with our settings.
>
> What it trains from is `../datasets/generated/data.yaml`. Unity writes that
> file at the end of every generation, listing only the runs it just made: the
> last generation set's run is **val** (the frames the model is scored on and
> never learns from), every earlier one is **train**. For the corpus of
> 2026-11-10 that is Set-01…15 for train (15,000 frames) and Set-16 for val
> (1,000 frames).
>
> > [!IMPORTANT]
> > Run from `__object_detection__/train/`
>
>> ### PowerShell
>> ```pwsh
>> uv run yolo detect train model=yolo11n.pt `
>>     data=D:/git/cart-pole/__object_detection__/datasets/generated/data.yaml `
>>     imgsz=640 `
>>     batch=-1 `
>>     epochs=500 `
>>     patience=50 `
>>     device=0 `
>>     project=D:/git/cart-pole/__object_detection__/train/runs `
>>     name=640-base
>> ```
>
>> ### What each setting means
>>
>> The values are the ones the old `training/` project trained the robot's
>> current model with (`training/04-COMMANDS.md`, the `cp2-640-base` run).
>>
>> - **`yolo detect train`**: `yolo` is Ultralytics' command, installed into
>>   `.venv/` by Step 1. `detect` is the task (boxes around objects), `train`
>>   the job.
>> - **`model=yolo11n.pt`**: the starting model, YOLO11 nano, the smallest
>>   YOLO11. It already knows how to find everyday objects, so training only
>>   has to teach it the four markers. If the file is not in this folder,
>>   Ultralytics downloads it (about 5 MB, from its GitHub releases) the first
>>   time.
>> - **`data=…/data.yaml`**: the list of train and val runs, and the four class
>>   names. Written out in full so the run never depends on the folder it was
>>   started from.
>> - **`imgsz=640`**: each frame is shrunk until its longer side is 640 pixels
>>   before the model sees it. 640 is the size the robot's computer runs the
>>   model at, so it is the size to train at.
>> - **`batch=-1`**: how many frames the model sees before each adjustment.
>>   `-1` lets Ultralytics measure the graphics card and pick the largest batch
>>   that fits in about 60% of its memory.
>> - **`epochs=500`**: an **epoch** is one pass over all 15,000 training
>>   frames. 500 is the most the run will do.
>> - **`patience=50`**: stop early if the val score has not improved for 50
>>   epochs in a row, since more passes are then only wasting time.
>> - **`device=0`**: train on the first graphics card, the RTX 3090.
>> - **`project=…/runs` `name=640-base`**: the results go in
>>   `runs/640-base/`. If that folder already exists, Ultralytics makes
>>   `runs/640-base2/` instead of overwriting it.
>
>> ### Expected Results:
>> While it runs, it prints one block per epoch: the losses (how wrong the
>> boxes and classes still are on the training frames, falling as it learns),
>> then the val scores per class. When it ends, the results are in
>> `D:/git/cart-pole/__object_detection__/train/runs/640-base/`, the `project=`
>> folder plus the `name=`:
>>
>> ```text
>> D:/git/cart-pole/__object_detection__/train/runs/640-base/
>> ├── weights/
>> │   ├── best.pt    the model from the epoch with the best val score: the one to use
>> │   └── last.pt    the model from the last epoch, to resume a stopped run
>> ├── results.csv    one row per epoch: every loss and val score
>> └── *.png          plots of the same, and example val frames with their boxes
>> ```
