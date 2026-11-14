# Update the real Orin — giving the robot the new detector

The robot balances the pole by finding its four markers in each camera frame.
Its computer, a Jetson **Orin** Nano Super, finds them with the detector
[02-TRAIN.md](02-TRAIN.md) trains. Training a new detector changes nothing on the
robot: the Orin keeps running the old one until it is given the new one. This
file copies the new model to the Orin, compiles it there, checks it through
the real camera, and restarts the two programs that use it.

> ## WEIGHTS — the run's name, set on each machine
>
> Every command below names the training run with one shell variable,
> `WEIGHTS`: the same name [02-TRAIN.md](02-TRAIN.md) trained it under. The steps
> run on two machines, the PC and the Orin, each in its own terminal. A
> variable lives only in the terminal it was set in, so open both terminals
> now, set `WEIGHTS` in each, and keep both open until the last step. Every
> step below names the terminal it runs in.
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
>> ### The Orin's terminal
>>
>> Open a second PowerShell and log in to the Orin with **`ssh`**, which gives
>> a terminal on another computer over the network. The Orin is `robo-orin`,
>> at `192.168.0.215`, user `tony`:
>>
>> ```pwsh
>> ssh tony@192.168.0.215
>> ```
>>
>> The prompt changes to the Orin's, `tony@robo-orin`. From here this window
>> is the Orin's bash, so set the variable here, after logging in:
>>
>> ```bash
>> WEIGHTS=640-21k
>> ```
>>
>> If this window is closed or the connection drops, log in again and set
>> `WEIGHTS` again: the new session starts without it.
>
> `${WEIGHTS}` is the same variable as `$WEIGHTS`. The braces mark where the
> name ends, needed when a letter, digit or `_` follows it: without them,
> `$WEIGHTS_fp16` would be read as a variable named `WEIGHTS_fp16`.
>
> In the folder tree and the expected results, which are not commands,
> `${WEIGHTS}` stands for its value: `640-21k` above.

The model these steps use is `train/runs/$WEIGHTS/weights/best.onnx` on the PC, the
file 02-TRAIN.md's Step 4 exported. Step 1 copies it to the Orin as
`~/orin-test/${WEIGHTS}.onnx`, and Step 4's script, `build_engine.py`, compiles
it there into `~/orin-test/${WEIGHTS}_fp16.engine`.

## Why there is a build step

The Orin does not run `best.onnx` itself. It runs a **TensorRT engine**: the
same model compiled by TensorRT, NVIDIA's engine for running models fast, for
one graphics chip and one TensorRT version. An engine built on this PC's RTX
3090 will not load on the Orin, so the Orin builds its own from the same
`best.onnx`. **FP16** means the engine works in 16-bit numbers instead of
32-bit, which is faster; it is the precision the robot runs at.

## What is where on the Orin

The Orin's terminal opens in its home folder, `~`. **`scp`**, run in the PC's
terminal, copies a file from the PC into these folders over the network.

```text
~/                              the Orin's home folder, /home/tony
├── orin-test/                  the detector's files (not in git)
│   ├── best.onnx               the old model, kept for going back
│   ├── best_fp16.engine        the old model's engine, kept for going back
│   ├── ${WEIGHTS}.onnx         the new model, copied by Step 1
│   └── ${WEIGHTS}_fp16.engine  the new model's engine, built by Step 4
├── live-orin-python/           a copy of __live__/orin/python: its settings.toml
│                               names the model and engine ([detector])
└── orin-cam/                   the webserver and the brain, which runs the balance loop
```

Two programs load the engine, each only when it starts, so both are restarted
(Steps 5 and 7):

- **the calibration service**, `~/live-orin-python/src/calibrate_service.py`,
  which measures the markers for the calibration and the live view;
- **the brain**, `~/orin-cam/target/release/brain`, whose Python worker
  detects the markers during a balance run.

> ## Step 1 — copy the model to the Orin (not run yet)
>
> The new name, `${WEIGHTS}.onnx`, leaves the old `best.onnx` where it is, so going
> back is only a settings change.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> scp "__object_detection__/train/runs/$WEIGHTS/weights/best.onnx" "tony@192.168.0.215:~/orin-test/${WEIGHTS}.onnx"
>> ```
>
> Then check the copy is whole: a **SHA-256 checksum** is a 64-character
> fingerprint of a file's bytes, and two files with the same one are the same
> file.
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> Get-FileHash -Algorithm SHA256 "__object_detection__/train/runs/$WEIGHTS/weights/best.onnx"
>> ```
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> sha256sum ~/orin-test/${WEIGHTS}.onnx
>> ```
>
>> ### Expected Results:
>> The same 64 characters from both. PowerShell prints them in capitals and
>> `sha256sum` in small letters; that is the only difference.

> ## Step 2 — point the Orin's settings at the new model (not run yet)
>
> The Orin finds the model and its engine through two lines of
> `__live__/orin/python/settings.toml`, in its `[detector]` table:
>
> ```toml
> model = "~/orin-test/<name>.onnx"
> engine = "~/orin-test/<name>_fp16.engine"
> ```
>
> A toml file cannot read the shell's `WEIGHTS`, so the command below writes
> its value into both lines for you, whatever run they named before. They are
> the only two lines in the file that start with `model =` and `engine =`, so
> nothing else changes. `input_size = 640` stays right: the model was trained
> and exported at 640 (02-TRAIN.md, Steps 3 and 4).
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> $settings = "__live__/orin/python/settings.toml"
>> (Get-Content -Raw $settings) `
>>     -replace '(?m)^model = ".*"', "model = `"~/orin-test/${WEIGHTS}.onnx`"" `
>>     -replace '(?m)^engine = ".*"', "engine = `"~/orin-test/${WEIGHTS}_fp16.engine`"" |
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
>> Show the two lines as they are now:
>>
>> ```pwsh
>> Select-String -Path $settings -Pattern '^(model|engine) ='
>> ```
>
>> ### Expected Results:
>> Two lines, each after the file's name and its line number, with the run's
>> name from `WEIGHTS` in both. For `640-21k`:
>>
>> ```text
>> model = "~/orin-test/640-21k.onnx"
>> engine = "~/orin-test/640-21k_fp16.engine"
>> ```
>>
>> If they show `~/orin-test/.onnx` and `~/orin-test/_fp16.engine`, with no
>> name, `WEIGHTS` was not set in this terminal (*WEIGHTS — the run's name,
>> set on each machine*, at the top): set it and run the command again.

> ## Step 3 — send the settings to the Orin (not run yet)
>
> The Orin reads its own copy, `~/live-orin-python/settings.toml`, not the
> PC's. This packs `__live__/orin/python` into one file, copies it over and
> unpacks it there. Files of the same name are replaced and nothing is
> deleted, so what only the Orin has (its lens and calibration results) stays.
>
> > [!WARNING]
> > The whole folder goes, so the PC's `saved/policy.npz` goes too, and Step
> > 7's brain runs it. If it is not the policy the robot should run, put the
> > right one back on the PC first ([02-DEPLOY.md](../__synthetic__/02-DEPLOY.md),
> > *Going back*). A policy that trained with another detector than the one
> > this file installs should not run with it: 02-DEPLOY.md's Step 2b
> > compares the two.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> tar --exclude=./.venv -czf target/live-orin-python.tgz -C __live__/orin/python .
>> ```
>
> `tar` is Windows' own (`C:\Windows\System32\tar.exe`), so PowerShell runs it
> with nothing to install. `-C __live__/orin/python .` packs that folder's
> contents, so they unpack straight into `~/live-orin-python` on the Orin.
> `--exclude=./.venv` leaves the PC's Python environment behind: it would not
> work on the Orin, which has its own.
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> scp target/live-orin-python.tgz tony@192.168.0.215:~
>> ```
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> mkdir -p ~/live-orin-python && tar -xzf ~/live-orin-python.tgz -C ~/live-orin-python
>> ```
>
>> ### Expected Results:
>> In the Orin's terminal, from any folder:
>>
>> ```bash
>> grep -E "^(model|engine) =" ~/live-orin-python/settings.toml
>> ```
>> shows `model = "~/orin-test/${WEIGHTS}.onnx"` and
>> `engine = "~/orin-test/${WEIGHTS}_fp16.engine"`.

> ## Step 4 — build the engine on the Orin (not run yet)
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> cd ~/live-orin-python && .venv/bin/python src/build_engine.py ~/orin-test/${WEIGHTS}_fp16.engine
>> ```
>
>> ### What each part means
>>
>> - **`.venv/bin/python`**: the Python environment in `~/live-orin-python`,
>>   made to see the Orin's own packages, because TensorRT is installed only
>>   for the Orin's system Python (`__live__/orin/python/readme-old.md`, *The
>>   marker detector*).
>> - **`src/build_engine.py`**: the script that builds the engine. It reads
>>   `model` from the `settings.toml` Step 3 sent.
>> - **`~/orin-test/${WEIGHTS}_fp16.engine`**: where the engine is written, the
>>   path Step 2 put in `engine`.
>
>> ### Expected Results:
>> The script builds the engine, then loads it back and checks its two
>> **tensors**, the arrays that go in and come out:
>>
>> ```text
>> TensorRT 10.3.<…>: /home/tony/orin-test/${WEIGHTS}.onnx -> /home/tony/orin-test/${WEIGHTS}_fp16.engine (FP16)
>> input  images: [1, 1, 640, 640] DataType.FLOAT
>> output output0: [1, 8, 8400] DataType.FLOAT
>> ok: /home/tony/orin-test/${WEIGHTS}_fp16.engine (<size> bytes)
>> ```
>>
>> - **`10.3`**: the TensorRT that comes with the Orin's JetPack 6.2.3.
>> - **`[1, 1, 640, 640]`**: one frame, one colour channel (greyscale, because
>>   `data.yaml` says `channels: 1`), 640 × 640 pixels.
>> - **`[1, 8, 8400]`**: 8400 candidate boxes, each with 8 numbers: 4 for the
>>   box (centre x, centre y, width, height) and 1 score for each of the 4
>>   classes. 8400 is how many places the model looks at a 640 frame:
>>   80 × 80 + 40 × 40 + 20 × 20 grid cells, at three scales.
>>
>> If either shape is different, the script stops with an error naming the
>> shape it expected, and writes no `ok:` line.

> ## Step 5 — stop the calibration service and the brain (not run yet)
>
> > [!WARNING]
> > Never during a balance run: press STOP on
> > <http://192.168.0.215:8080/run/> first.
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> pkill -f calibrate_service.py; pkill -x brain
>> ```
>
> `pkill -f calibrate_service.py` ends the program whose command line names the
> service; `pkill -x brain` ends the program named exactly `brain`. Both stop
> here so the camera is free for Step 6.

> ## Step 6 — check the new engine through the real camera (not run yet)
>
> `live_detect.py` runs the detector on the camera's frames and prints where it
> found each marker. Nothing moves: it only watches.
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> cd ~/live-orin-python && .venv/bin/python src/live_detect.py 120 /tmp/live.jpg
>> ```
>
>> ### What each part means
>>
>> - **`120`**: how many frames to detect in, two seconds of the camera's
>>   60 frames/s.
>> - **`/tmp/live.jpg`**: the first frame is saved here, to look at.
>
>> ### Expected Results:
>> One line per frame, each marker with its centre in pixels and its score, or
>> `-` where it was not found; then the totals:
>>
>> ```text
>> 120 frames in <…> s: <…> frames/s. Found: Left 120, Right 120, Pivot 120, End 120
>> encode + detect ms/frame: mean <…>, max <…>
>> ```
>>
>> The old model found all four markers in 120 of 120 frames on 2026-11-01
>> (`__live__/orin/python/readme-old.md`, *The marker detector*). A marker
>> found in fewer means the new model sees the rig worse than the old one:
>> stop here and go back (*Going back*, below).

> ## Step 7 — start the calibration service and the brain (not run yet)
>
> Each keeps running after the SSH session ends: `setsid` gives it a session of
> its own, `nohup` keeps it running when this one closes, `< /dev/null` gives it
> no keyboard, and `> … 2>&1` sends everything it prints into a file. Neither
> starts by itself after the Orin reboots.
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal) — the calibration service
>> ```bash
>> cd ~/live-orin-python && (nohup setsid .venv/bin/python src/calibrate_service.py > service.out 2>&1 < /dev/null &)
>> ```
>
>> ### bash (the Orin's terminal) — the brain
>> ```bash
>> (
>>   cd ~/orin-cam &&
>>   nohup setsid ./target/release/brain > brain.out 2>&1 < /dev/null &
>> )
>> ```
>
>> ### Expected Results:
>> After a few seconds, each file names the new engine or says it is ready:
>>
>> ```bash
>> cat ~/live-orin-python/service.out
>> ```
>>
>> ends with `detector loaded: /home/tony/orin-test/${WEIGHTS}_fp16.engine`.
>>
>> ```bash
>> cat ~/orin-cam/brain.out
>> ```
>>
>> has a `worker ready:` line naming the policy, then
>> `brain idle at http://127.0.0.1:8091` (`__live__/orin/rust/DEPLOY.md`,
>> step 9). A line `Error: Worker { message: "…" … }` instead means the
>> worker could not start, and `message` says why; if it says the worker
>> ended before its ready line, the worker's own message is above it in the
>> file.

## Going back

Every earlier model and its engine are still in `~/orin-test/`, each named
after its run. To run one again, set `WEIGHTS` in the PC's terminal to that
run's name, then repeat Step 2, Step 3, Step 5 and Step 7. Its engine needs
no rebuild: it was built on this Orin.

The model from before the first update is `best.onnx` with
`best_fp16.engine`, so for it the name is `best`:

```pwsh
$WEIGHTS = "best"
```
