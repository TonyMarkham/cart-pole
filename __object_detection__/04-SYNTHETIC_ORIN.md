# Update the synthetic Orin — giving the training loop the new detector

The balance policy learns in the **synthetic loop**: MuJoCo moves a simulated
rig, Unity renders what the robot's camera would see, and the **synthetic
Orin** finds the markers in each frame, just as the real Orin does on the rig.
The synthetic Orin finds them with the detector [02-TRAIN.md](02-TRAIN.md) trains.
Training a new detector does not change what the loop uses: until these steps
are done, the loop still loads the old one. This file switches it to the new
one.

The synthetic Orin runs on this PC, not on the robot. Its code is
`__synthetic__/orin/python/`, and the trainer starts it, so nothing is copied
to another computer, and every step runs in one terminal on the PC.

> ## WEIGHTS — the run's name, set in the PC's terminal
>
> Every command below names the training run with one shell variable,
> `WEIGHTS`: the same name [02-TRAIN.md](02-TRAIN.md) trained it under. A variable
> lives only in the terminal it was set in, so open the terminal now, set
> `WEIGHTS` in it, and keep it open until the last step. If it is closed, open
> a new one and do this again: the new one starts without it.
>
>> ### The PC's terminal
>>
>> Open PowerShell and go to the repo root:
>>
>> ```pwsh
>> cd D:/git/cart-pole
>> ```
>>
>> ```pwsh
>> $WEIGHTS = "640-21k"
>> ```
>
> `${WEIGHTS}`, in Steps 1 and 3, is the same variable as `$WEIGHTS`: the
> braces only mark where the name ends.
>
> In the expected results, which are not commands, `$WEIGHTS` stands for its
> value: `640-21k` above.

The model these steps use is `train/runs/$WEIGHTS/weights/best.onnx`, the file
02-TRAIN.md's Step 4 exported. Step 2's script, `build_engine.py`, compiles it
into `train/runs/$WEIGHTS/weights/best_fp16.engine`, beside it.

## Why there is a build step

The synthetic Orin does not run `best.onnx` itself. It runs a **TensorRT
engine**: the same model compiled by TensorRT, NVIDIA's engine for running
models fast, for one graphics card and one TensorRT version. An engine built
for the Orin will not load on this PC's RTX 3090, nor the other way round, so
each machine builds its own from the same `best.onnx`. **FP16** means the
engine works in 16-bit numbers instead of 32-bit, which is faster; it is the
precision the robot runs at.

Two settings files name the model, and both must change:

| File | Setting | What it names |
|---|---|---|
| `__synthetic__/orin/python/settings.toml` | `model` | the `best.onnx` the engine is built from |
| `__synthetic__/trainer/python/settings.toml` | `--engine` in `command` | the engine the synthetic Orin loads during training |

A toml file cannot read the shell's `WEIGHTS`, so Steps 1 and 3 each run a
command that writes its value into the one line that names the run, whatever
run it named before.

Until 2026-11-11 both pointed into the repo root's `training/` folder, which
was removed that day, so the synthetic loop cannot start until Step 1 and
Step 3 are done.

> ## Step 1 — point the synthetic Orin at the new model (not run yet)
>
> The synthetic Orin finds the model through one line of
> `__synthetic__/orin/python/settings.toml`:
>
> ```toml
> model = "../../../__object_detection__/train/runs/<name>/weights/best.onnx"
> ```
>
> A relative path in that file is read from the file's own folder,
> `__synthetic__/orin/python/`, so `../../../` is the repo root. It is the only
> line in the file that starts with `model =`, so nothing else changes.
> `input_size = 640` stays right: the model was trained and exported at 640
> (02-TRAIN.md, Steps 3 and 4).
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> $settings = "__synthetic__/orin/python/settings.toml"
>> (Get-Content -Raw $settings) `
>>     -replace '(?m)^model = ".*"', "model = `"../../../__object_detection__/train/runs/${WEIGHTS}/weights/best.onnx`"" |
>>     Set-Content -NoNewline $settings
>> ```
>
>> ### What each part means
>>
>> - **`Get-Content -Raw`**: reads the whole file as one piece of text, line
>>   endings included, so they are written back unchanged.
>> - **`-replace '<pattern>', "<new text>"`**: finds the text the pattern
>>   describes and puts the new text in its place. In the pattern, `(?m)^`
>>   means "at the start of a line", and `".*"` means "a quoted value,
>>   whatever it is", so the old run's name does not need to be known.
>> - **`` `" ``**: a `"` inside the new text. The backtick tells PowerShell
>>   this quote is part of the text, not its end. `${WEIGHTS}` is filled in
>>   with the run's name before the replace happens.
>> - **`Set-Content -NoNewline`**: writes the text back to the same file,
>>   adding no extra line at its end.
>
>> ### Check the file
>>
>> Show the line as it is now:
>>
>> ```pwsh
>> Select-String -Path $settings -Pattern '^model ='
>> ```
>
>> ### Expected Results:
>> One line, after the file's name and its line number, with the run's name
>> from `WEIGHTS` in it. For `640-21k`:
>>
>> ```text
>> model = "../../../__object_detection__/train/runs/640-21k/weights/best.onnx"
>> ```
>>
>> If it shows `runs//weights`, with no name, `WEIGHTS` was not set in this
>> terminal (*WEIGHTS — the run's name, set in the PC's terminal*, at the
>> top): set it and run the command again.

> ## Step 2 — build the engine (not run yet)
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/build_engine.py `
>>     "__object_detection__/train/runs/$WEIGHTS/weights/best_fp16.engine"
>> ```
>
>> ### What each part means
>>
>> - **`uv run --project __synthetic__/orin/python`**: run inside the synthetic
>>   Orin's own Python environment, the one with TensorRT in it (its
>>   `pyproject.toml` lists `tensorrt-cu12`). The first time, uv installs it.
>> - **`python …/src/build_engine.py`**: the script that builds the engine. It
>>   reads `model` from the `settings.toml` Step 1 changed.
>> - **`…/best_fp16.engine`**: where the engine is written, beside the
>>   `best.onnx` it comes from. Git ignores it (the root `.gitignore` keeps only
>>   `best.pt` and `best.onnx` of a run), because it only works on this PC.
>
>> ### Expected Results:
>> The script builds the engine, then loads it back and checks its two
>> **tensors**, the arrays that go in and come out:
>>
>> ```text
>> TensorRT <version>: D:\git\cart-pole\__object_detection__\train\runs\$WEIGHTS\weights\best.onnx -> __object_detection__\train\runs\$WEIGHTS\weights\best_fp16.engine (FP16)
>> input  images: [1, 1, 640, 640] DataType.FLOAT
>> output output0: [1, 8, 8400] DataType.FLOAT
>> ok: __object_detection__\train\runs\$WEIGHTS\weights\best_fp16.engine (<size> bytes)
>> ```
>>
>> - **`[1, 1, 640, 640]`**: one frame, one colour channel (greyscale, because
>>   `data.yaml` says `channels: 1`), 640 × 640 pixels.
>> - **`[1, 8, 8400]`**: 8400 candidate boxes, each with 8 numbers: 4 for the
>>   box (centre x, centre y, width, height) and 1 score for each of the 4
>>   classes. 8400 is how many places the model looks at a 640 frame:
>>   80 × 80 + 40 × 40 + 20 × 20 grid cells, at three scales.
>>
>> If either shape is different, the script stops with an error naming the
>> shape it expected, and writes no `ok:` line.

> ## Step 3 — point the trainer at the new engine (not run yet)
>
> The trainer starts the whole loop with the `command` list in
> `__synthetic__/trainer/python/settings.toml`. Its last argument, `--engine`,
> is the engine the synthetic Orin loads, on one line of that list:
>
> ```toml
>     "--engine", "__object_detection__/train/runs/<name>/weights/best_fp16.engine",
> ```
>
> The paths in `command` are read from the repo root
> (`[orchestrator] cwd = "../../.."`). It is the only line in the file that
> holds `"--engine",`, so nothing else changes.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> $settings = "__synthetic__/trainer/python/settings.toml"
>> (Get-Content -Raw $settings) `
>>     -replace '(?m)^([ \t]*"--engine", )".*"', "`$1`"__object_detection__/train/runs/${WEIGHTS}/weights/best_fp16.engine`"" |
>>     Set-Content -NoNewline $settings
>> ```
>
>> ### What each part means
>>
>> The same as Step 1's command, with one addition:
>>
>> - **`([ \t]*"--engine", )`** and **`` `$1 ``**: the brackets keep the
>>   start of the line, its spaces and `"--engine", `, and `$1` puts that
>>   kept part back in front of the new path, so only the path changes. The
>>   backtick before `$1` stops PowerShell reading `$1` as one of its own
>>   variables.
>
>> ### Check the file
>>
>> Show the line as it is now:
>>
>> ```pwsh
>> Select-String -Path $settings -Pattern '"--engine"'
>> ```
>
>> ### Expected Results:
>> One line, after the file's name and its line number, with the run's name
>> from `WEIGHTS` in it. For `640-21k`:
>>
>> ```text
>>     "--engine", "__object_detection__/train/runs/640-21k/weights/best_fp16.engine",
>> ```
>>
>> If it shows `runs//weights`, with no name, `WEIGHTS` was not set in this
>> terminal: set it and run the command again.
>>
>> The next training run's synthetic Orin loads the new engine. It calibrates at
>> the start of every run (`__documentation__/synthetic-loop-dia.md`,
>> *Calibration*), so the calibration is done with the new detector too, with
>> nothing more to do.
>>
>> Build the engine again (Step 2) whenever this PC's graphics card or the
>> TensorRT version changes: an engine only loads on the graphics card and
>> TensorRT version it was built with (`build_engine.py`'s own header).

## Going back

Every earlier run's model and engine stay in its own `train/runs/<name>/weights/`.
To use one again, set `WEIGHTS` in the PC's terminal to that run's name, then
repeat Step 1 and Step 3. Its engine needs no rebuild, unless this PC's
graphics card or TensorRT version has changed since it was built: then repeat
Step 2 too.

The model from before the first update, `training/runs/cp2-640-base/`, cannot
be gone back to: its runs were never in git, and they are no longer on this
PC's disk.
