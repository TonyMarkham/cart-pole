# 02-DEPLOY — giving the robot the new policy

The robot balances the pole with the policy: the small network that reads
where the markers were in the last six frames, and its own last six commands,
and picks the cart's next speed. Training a new policy changes nothing on the
robot: its computer, a Jetson **Orin** Nano Super, keeps running the old one
until it is given the new one. This file writes the newest training
session's policy into the file the robot runs, checks it, copies it to the
Orin, checks it again there, and restarts the one program that uses it, the
**brain**, which runs the balance loop.

It is the second of three files, each starting where the one before ends. It
starts where [01-TRAIN.md](01-TRAIN.md) ends, with a trained session in
`__synthetic__/trainer/python/runs/`, and ends with the brain running the new
policy, where [03-RUN.md](03-RUN.md), one balance run on the rig, begins.
Keeping the new policy, or going back to the one before, comes after that
run (*Keeping it* and *Going back*, at the end).

The policy these steps use is `__live__/orin/python/saved/policy.npz` on the
PC. It is one numpy file (**numpy** is Python's library for arrays of
numbers; an `.npz` is its file of named arrays): the network's numbers, the
values it was trained with, and 1,000 of the session's own inputs with the
trainer's answer to each, so any copy of it can be checked against training.
The 1,000 is `check_samples` in `[export_policy]` of
`__synthetic__/trainer/python/measures.toml`.

> [!IMPORTANT]
> A training session that has finished is not in this file yet. Training
> writes only into the session's own folder,
> `__synthetic__/trainer/python/runs/<date>_<time>/`. Step 1 below, the
> **export**, is what writes the newest session's policy into `policy.npz`;
> until it runs, the file still holds an older session's policy.

## Two terminals, and SESSION — the training session's name

The steps run on two machines, the PC and the Orin, each in its own terminal.
Open both now and keep both open until the last step. Every step names the
terminal it runs in. A step whose title ends in *(not run yet)* has not yet
been run as written here.

> ### The PC's terminal
>
> Open PowerShell and go to the repo root, the folder with `Cargo.toml` and
> `AGENTS.md` in it:
>
> > [!CAUTION]
> > Not copy/paste on another computer: `D:/git/cart-pole` is where the repo
> > is on the author's PC. Replace it with the folder you cloned the repo into.
>
> ```pwsh
> cd D:/git/cart-pole
> ```
>
> Then set one shell variable, `SESSION`: the newest training session that has
> a save, the `<date>_<time>` name of its folder in
> `__synthetic__/trainer/python/runs/`. It is the same variable, set by the
> same command, as in 01-TRAIN.md (*SESSION — the training session's name*).
> A variable lives only in the terminal it was set in: if this window is
> closed, open a new one, go to the repo root and set it again. Set it here
> even if this window already has it from 01-TRAIN.md or from *Going back*:
> this is the value these steps need.
>
> ```pwsh
> $SESSION = (Get-ChildItem __synthetic__/trainer/python/runs -Directory | Where-Object { Test-Path (Join-Path $_.FullName "latest.json") } | Sort-Object Name | Select-Object -Last 1).Name
> ```
>
>> ### What each part means
>>
>> - **`Get-ChildItem … -Directory`**: the folders in `runs/`, one per session.
>> - **`Where-Object { Test-Path … "latest.json" }`**: keeps only the folders
>>   that hold a save, `latest.json` with `latest.zip`. A session gets its
>>   folder the moment it starts, and saves every `[train] save_every` steps
>>   (`__synthetic__/trainer/python/gym.toml`, 10,000), and again whenever it
>>   ends: at its total, on Ctrl+C, or on a failure (`train.py`, its
>>   `finally`). Only a session that failed before training began has no
>>   save.
>> - **`Sort-Object Name`**: puts them in order of their names. A name is the
>>   date and then the time, the largest unit first, so this is also the order
>>   the sessions were started in.
>> - **`Select-Object -Last 1`**: the last of them, the newest session with a
>>   save.
>> - **`.Name`**: its folder's name alone, without the path before it.
>
>> ### Check it
>> ```pwsh
>> $SESSION
>> ```
>
>> ### Expected Results:
>> The newest session's name: on 2026-11-14, `20261113_074052`. This is the
>> session these steps deploy. An empty line means no session in `runs/` has
>> a save yet.
>
> So the newest session with a save is not always the one you mean: a session
> still training, or a short test session started later and stopped, has a
> save too. Check how far its save got:
>
> ```pwsh
> (Get-Content "__synthetic__/trainer/python/runs/$SESSION/latest.json" | ConvertFrom-Json).steps
> ```
>
>> ### Expected Results:
>> The save's step count, `6000000` for `20261113_074052`: the session's
>> `[train] total_steps` (`gym.toml`, 6,000,000), so it trained to the end.
>>
>> > [!WARNING]
>> > A smaller number means `SESSION` is a session that is still training or
>> > was stopped early. If that is not the session you mean to deploy, stop
>> > here and name the one you mean instead:
>> >
>> > > [!CAUTION]
>> > > Not copy/paste: replace `<date>_<time>` with the name of the session's
>> > > folder in `__synthetic__/trainer/python/runs/`.
>> >
>> > ```pwsh
>> > $SESSION = "<date>_<time>"
>> > ```
>> >
>> > Then run the step count again.
>
> In the expected results below, which are not commands, `${SESSION}` stands
> for its value.

> ### The Orin's terminal
>
> Open a second PowerShell and log in to the Orin with **`ssh`**, which gives
> a terminal on another computer over the network. The Orin is `robo-orin`,
> at `192.168.0.215`, user `tony`:
>
> > [!CAUTION]
> > Not copy/paste on another rig: `tony@192.168.0.215` is the author's Orin,
> > its user and its address on the network. Replace it with your Orin's, here,
> > in every `scp` and `ssh` command below, and in the `http://192.168.0.215`
> > link of Step 6.
>
> ```pwsh
> ssh tony@192.168.0.215
> ```
>
> The prompt changes to the Orin's, `tony@robo-orin`. From here this window
> is the Orin's bash.

## What is where on the Orin

The Orin's terminal opens in its home folder, `~`. **`scp`**, run in the PC's
terminal, copies a file from the PC into these folders over the network.

```text
~/                              the Orin's home folder, /home/tony
├── live-orin-python/           a copy of __live__/orin/python
│   ├── saved/policy.npz        the policy the brain runs, replaced by Step 4
│   └── src/test_policy.py      the policy's check, run by Step 5
└── orin-cam/                   the webserver and the brain
    └── target/release/brain    the brain, restarted by Steps 6 and 7
```

Only the brain loads the policy, and only when it starts, through its Python
worker, `~/live-orin-python/src/brain_worker.py`. So the brain alone is
restarted. The Orin's other Python program, the **calibration service**
(`~/live-orin-python/src/calibrate_service.py`, which measures the markers
for the calibration and the live view), never reads the policy. It does read
the detector's engine from `settings.toml`, which Step 4 replaces, once, when
it starts. If the `engine` line changed since it was started, restart it
after Step 4: stop it with `pkill -f calibrate_service.py`, then start it
with the first command of
[03-REAL_ORIN.md](../__object_detection__/03-REAL_ORIN.md)'s Step 7.

> ## Step 1 — export `SESSION`'s policy into `policy.npz` (not run yet)
>
> Training saves the policy in the format of **Stable-Baselines3**, the Python
> library the trainer learns with, which needs **PyTorch**, a large maths
> library. The robot's computer runs the policy with numpy alone. So this
> writes the one part the robot needs, the network that picks the command,
> from the session `SESSION` names into `policy.npz`, replacing the file
> whole. The policy it replaces stays in git (*Going back*, at the end).
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/export_policy.py "__synthetic__/trainer/python/runs/$SESSION" --out __live__/orin/python/saved/policy.npz
>> ```
>
>> ### What each part means
>>
>> - **`export_policy.py`**: the trainer's export script, in
>>   `__synthetic__/trainer/python/src/`.
>> - **`"__synthetic__/trainer/python/runs/$SESSION"`**: the session's folder.
>>   The export reads its newest save, `latest.zip`, and the steps it learned
>>   from, `latest_buffer.pkl`.
>> - **`--out __live__/orin/python/saved/policy.npz`**: the file to write, the
>>   one the robot runs.
>
> Before writing, it checks its own arithmetic. The session's **replay
> buffer** is the store of recent steps the trainer learned from, each with
> what the policy saw; it keeps the last `buffer_size` steps (`[sac]` in
> `gym.toml`, 2,000,000), shared out evenly between the session's 12
> simulated robots, so 166,666 each, 1,999,992 in all. For every one of them
> the export works out the command both through Stable-Baselines3 and through
> the numpy formula the robot uses, and refuses to write if any two differ by
> more than 0.00001 (`TOLERANCE` in `export_policy.py`). For
> `20261113_074052` it took 14 s on this PC (2026-11-14).
>
>> ### Expected Results:
>> Three lines: the save it read, how many steps it compared and the largest
>> difference, and the file it wrote. With your repo's folder in place of
>> `D:\git\cart-pole`:
>>
>> ```text
>> export_policy: D:\git\cart-pole\__synthetic__\trainer\python\runs\${SESSION}\latest.zip
>> export_policy: <count> buffer inputs, worst difference <difference>
>> export_policy: wrote D:\git\cart-pole\__live__\orin\python\saved\policy.npz
>> ```
>>
>> For `20261113_074052`, on 2026-11-14, the middle line was
>> `export_policy: 1,999,992 buffer inputs, worst difference 6.05e-06`.
>>
>> > [!WARNING]
>> > Anything other than a last line starting `export_policy: wrote` means
>> > the export failed and `policy.npz` was not changed: either the refusal
>> > `export_policy: the formula differs from Stable-Baselines3 by …`, or a
>> > Python error. Stop here, and do not go on to Step 2.

> ## Step 2 — check the policy and the robot's detector (not run yet)
>
>> ### 2a. Check the policy file is `SESSION`'s
>>
>> The export writes the name of the session it came from into the file. This
>> reads that name back and compares it with `SESSION`, so it shows Step 1
>> wrote that session's policy.
>>
>> > [!IMPORTANT]
>> > Run in the PC's terminal, from the repo root
>>
>> ```pwsh
>> (uv run --project __synthetic__/trainer/python python -c "import numpy; print(str(numpy.load('__live__/orin/python/saved/policy.npz')['source']).split('/')[0])") -eq $SESSION
>> ```
>>
>>> #### What each part means
>>>
>>> - **`uv run --project __synthetic__/trainer/python python -c "…"`**: runs
>>>   the Python one-liner in quotes with the trainer's packages, the same ones
>>>   Step 1 used, numpy among them.
>>> - **`numpy.load(…)['source']`**: the line the export wrote into the file
>>>   to say where it came from, in the form
>>>   `<date>_<time>/latest.zip, <steps> steps, commit …`.
>>> - **`.split('/')[0]`**: the part of that line before its first `/`: the
>>>   name of the session the file came from.
>>> - **`( … ) -eq $SESSION`**: `True` when that name is `SESSION`'s, `False`
>>>   when it is not.
>>
>>> #### Expected Results:
>>> ```text
>>> True
>>> ```
>>>
>>> > [!WARNING]
>>> > `False` means `policy.npz` is not `SESSION`'s export. Stop here, and do
>>> > not go on. There are two causes:
>>> >
>>> > - `SESSION` is not set in this window: `$SESSION` prints an empty line.
>>> >   Set it (*Two terminals, and SESSION*, at the top), then run Step 1 and
>>> >   this check again.
>>> > - Step 1 did not write the file: its last line was not
>>> >   `export_policy: wrote …`. Its warning applies; running it again
>>> >   unchanged gives the same failure.
>
>> ### 2b. Check the robot runs the detector the policy trained with
>>
>> The policy learned from what one detector found in each frame, so the
>> robot should run that same detector. A detector is named by its training
>> run in `__object_detection__/train/runs/`. The session recorded which
>> engine it trained with in its `settings_from_<steps>.toml` files, one for
>> each start (01-TRAIN.md, Step 5's table); the robot's is the `engine` line
>> of `__live__/orin/python/settings.toml`, which Step 4 sends to the Orin.
>> This prints both.
>>
>> > [!IMPORTANT]
>> > Run in the PC's terminal, from the repo root
>>
>> ```pwsh
>> Select-String -Path "__synthetic__/trainer/python/runs/$SESSION/settings_from_*.toml" -Pattern '"--engine", "([^"]*)"' | ForEach-Object { "trained with: " + $_.Matches[0].Groups[1].Value }
>> Select-String -Path __live__/orin/python/settings.toml -Pattern '^engine = "([^"]*)"' | ForEach-Object { "robot runs:   " + $_.Matches[0].Groups[1].Value }
>> ```
>>
>>> #### What each part means
>>>
>>> - **`Select-String -Path … -Pattern '…'`**: finds the lines of those
>>>   files that match the pattern.
>>> - **`"([^"]*)"`**: the part of the pattern in brackets is the path
>>>   between the quotes; `$_.Matches[0].Groups[1].Value` takes it.
>>
>>> #### Expected Results:
>>> The same detector's name in every line. For `20261113_074052`, on
>>> 2026-11-14:
>>>
>>> ```text
>>> trained with: __object_detection__/train/runs/640-21k/weights/best_fp16.engine
>>> robot runs:   ~/orin-test/640-21k_fp16.engine
>>> ```
>>>
>>> Both name `640-21k`.
>>>
>>> > [!WARNING]
>>> > Different names mean the policy would run with a detector it never
>>> > trained with. Stop here. Either train a new session with the robot's
>>> > detector (01-TRAIN.md), or give the robot this session's detector
>>> > ([03-REAL_ORIN.md](../__object_detection__/03-REAL_ORIN.md)).

> ## Step 3 — check the policy on the PC
>
> `test_policy.py` works out the command for the 1,000 inputs stored in the
> file, with numpy alone, the way the robot does, and compares each with the
> trainer's own answer. A file that was written wrongly, or that the robot's
> code would read differently from training, fails here, before it goes
> anywhere.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> uv run --project __live__/orin/python python __live__/orin/python/src/test_policy.py
>> ```
>
>> ### Expected Results:
>> ```text
>> Ran 6 tests in <…>s
>>
>> OK
>> ```
>>
>> Five of the tests use a network small enough to work out by hand: two
>> check the arithmetic (the network's answer, and the command it becomes),
>> one checks the comparison with the trainer's answers, and two check that a
>> file for the wrong inputs, or no file, is refused. The sixth checks
>> `saved/policy.npz` itself against the trainer's answers.
>
> The first time this runs, `uv` makes `__live__/orin/python/.venv` and
> installs that project's packages into it (its `pyproject.toml`: numpy,
> OpenCV and NVIDIA's cuda-python); later runs reuse it.

> ## Step 4 — copy the policy to the Orin (not run yet)
>
> This packs `__live__/orin/python` into one file, copies it over and unpacks
> it into `~/live-orin-python`, as
> [03-REAL_ORIN.md](../__object_detection__/03-REAL_ORIN.md)'s Step 3 does for
> the detector's settings. Files of the same name are replaced and nothing is
> deleted, so what only the Orin has (its lens and calibration results)
> stays.
>
> > [!IMPORTANT]
> > The whole folder goes, not only the policy: its code, and its
> > `settings.toml`, which replaces the Orin's. That file's `[detector]`
> > names the TensorRT engine the brain's worker loads, now
> > `~/orin-test/640-21k_fp16.engine`. The engine is not in the folder: it is
> > built on the Orin by 03-REAL_ORIN.md's Step 4. Sub-step 4e shows whether
> > the Orin has it.
>
>> ### 4a. Pack the folder into one file
>>
>> > [!IMPORTANT]
>> > Run in the PC's terminal, from the repo root
>>
>> ```pwsh
>> New-Item -ItemType Directory -Force target | Out-Null
>> tar --exclude=./.venv -czf target/live-orin-python.tgz -C __live__/orin/python .
>> ```
>>
>>> #### What each part means
>>>
>>> - **`New-Item … target`**: makes the repo's `target/` folder if it is not
>>>   there yet (it is not on a fresh clone until something is built);
>>>   `| Out-Null` hides what it prints.
>>> - **`tar`**: Windows' own (`C:\Windows\System32\tar.exe`), so PowerShell
>>>   runs it with nothing to install.
>>> - **`-C __live__/orin/python .`**: packs that folder's contents, so they
>>>   unpack straight into `~/live-orin-python` on the Orin.
>>> - **`--exclude=./.venv`**: leaves the PC's Python environment behind: it
>>>   would not work on the Orin, which has its own.
>
>> ### 4b. Copy the file to the Orin
>>
>> > [!IMPORTANT]
>> > Run in the PC's terminal, from the repo root
>>
>> > [!CAUTION]
>> > Not copy/paste on another rig: replace `tony@192.168.0.215` with your
>> > Orin's user and address.
>>
>> ```pwsh
>> scp target/live-orin-python.tgz tony@192.168.0.215:~
>> ```
>
>> ### 4c. Unpack it on the Orin
>>
>> > [!IMPORTANT]
>> > Run in the Orin's terminal, from `~`
>>
>> ```bash
>> mkdir -p ~/live-orin-python && tar -xzf ~/live-orin-python.tgz -C ~/live-orin-python
>> ```
>
>> ### 4d. Check the copy is whole
>>
>> A **SHA-256 checksum** is a 64-character fingerprint of a file's bytes, and
>> two files with the same one are the same file. Work it out on both
>> machines.
>>
>> > [!IMPORTANT]
>> > Run in the PC's terminal, from the repo root
>>
>> ```pwsh
>> Get-FileHash -Algorithm SHA256 __live__/orin/python/saved/policy.npz
>> ```
>>
>> > [!IMPORTANT]
>> > Run in the Orin's terminal, from `~`
>>
>> ```bash
>> sha256sum ~/live-orin-python/saved/policy.npz
>> ```
>>
>>> #### Expected Results:
>>> The same 64 characters from both. PowerShell prints them in capitals and
>>> `sha256sum` in small letters; that is the only difference.
>
>> ### 4e. Check the Orin has the engine its new `settings.toml` names
>>
>> > [!IMPORTANT]
>> > Run in the Orin's terminal, from `~`
>>
>> ```bash
>> engine=$(sed -n 's/^engine = "\(.*\)"/\1/p' ~/live-orin-python/settings.toml); engine=${engine/#\~/$HOME}; if [ -f "$engine" ]; then echo "engine found: $engine"; else echo "engine missing: $engine"; fi
>> ```
>>
>>> #### What each part means
>>>
>>> - **`sed -n 's/^engine = "\(.*\)"/\1/p' …`**: prints the path between the
>>>   quotes on the line that starts `engine = `, and nothing else.
>>> - **`engine=${engine/#\~/$HOME}`**: turns the `~` at its start into the
>>>   home folder's full path, `/home/tony`, since the test below does not
>>>   read `~` inside quotes.
>>> - **`[ -f "$engine" ]`**: true when that file exists.
>>
>>> #### Expected Results:
>>> ```text
>>> engine found: /home/tony/orin-test/640-21k_fp16.engine
>>> ```
>>>
>>> > [!WARNING]
>>> > `engine missing: …` means the brain's worker would not start. Stop
>>> > here, before Step 6 stops the running brain, and build that engine
>>> > first ([03-REAL_ORIN.md](../__object_detection__/03-REAL_ORIN.md),
>>> > Step 4).

> ## Step 5 — check the policy on the Orin (not run yet)
>
> The same check as Step 3, now with the Orin's own numpy, on the copy the
> brain will load.
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> (cd ~/live-orin-python && .venv/bin/python src/test_policy.py)
>> ```
>
>> ### What each part means
>>
>> - **`( … )`**: runs the commands inside in a copy of the shell, so the `cd`
>>   does not move this terminal: it stays in `~` for the steps after.
>> - **`.venv/bin/python`**: the Python environment in `~/live-orin-python`,
>>   made on the Orin to see the Orin's own packages, because TensorRT is
>>   installed only for the Orin's system Python. It is made once, by the
>>   command in `__live__/orin/python/readme-old.md` (*The detector needs the
>>   `.venv` to see the system's packages*); a later copy in Step 4 leaves it
>>   alone.
>
>> ### Expected Results:
>> As in Step 3: `Ran 6 tests`, then `OK`.

> ## Step 6 — stop the brain (not run yet)
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
>> pkill -x brain
>> ```
>
> `pkill -x brain` ends the program named exactly `brain`. Its worker, which
> holds the old policy, ends with it.

> ## Step 7 — start the brain (not run yet)
>
> This starts the brain, which loads the new policy as it starts, reads what
> it printed on the Orin, then checks from the PC that it runs this session's
> policy.
>
>> ### 7a. Start the brain
>>
>> > [!IMPORTANT]
>> > Run in the Orin's terminal, from `~`
>>
>> ```bash
>> (
>>   cd ~/orin-cam &&
>>   nohup setsid ./target/release/brain > brain.out 2>&1 < /dev/null &
>> )
>> ```
>>
>>> #### What each part means
>>>
>>> The brain keeps running after the SSH session ends. It does not start by
>>> itself after the Orin reboots.
>>>
>>> - **`setsid`**: gives it a session of its own.
>>> - **`nohup`**: keeps it running when this one closes.
>>> - **`< /dev/null`**: gives it no keyboard.
>>> - **`> brain.out 2>&1`**: sends everything it prints into
>>>   `~/orin-cam/brain.out`, which 7b and 7c read.
>
>> ### 7b. Read what it printed
>>
>> > [!IMPORTANT]
>> > Run in the Orin's terminal, from `~`
>>
>> ```bash
>> cat ~/orin-cam/brain.out
>> ```
>>
>>> #### Expected Results:
>>> After a few seconds (the worker is given up to 60 s to load,
>>> `worker_start_timeout_ms` in `__live__/orin/rust/settings.toml`), among
>>> its lines, a `worker ready:` line naming the new policy's training
>>> session, its save, its step count and the commit it was trained at, then
>>> the brain waiting for a run:
>>>
>>> ```text
>>> worker ready: policy ${SESSION}/latest.zip, <steps> steps, commit <commit>, top rate 44600 pulses/s, 6 past commands, ages in units of 1000 ms
>>> brain idle at http://127.0.0.1:8091
>>> ```
>>>
>>> `<commit>` is the first 8 characters of the repo's commit the session
>>> trained at, followed by ` with uncommitted changes` if the repo also had
>>> changes not yet committed then (`export_policy.py` writes both from the
>>> save's `latest.json`): for `20261113_074052`,
>>> `commit 747d9d0f with uncommitted changes`. The file has other lines too:
>>> the worker's own `brain worker: engine loaded, policy …`, and any
>>> warnings TensorRT prints.
>>>
>>> > [!WARNING]
>>> > A line starting `Error:` instead means the brain stopped:
>>> >
>>> > - `Error: Worker { message: "…" … }`: the worker could not start, and
>>> >   `message` says why. If it says the worker ended before its ready
>>> >   line, the worker's own message is above it in the file.
>>> > - `Error: Io { message: "…" … }` after the `worker ready:` line: most
>>> >   likely the old brain still holds its address, so Step 6 did not stop
>>> >   it.
>
>> ### 7c. Check from the PC that the brain runs this session's policy
>>
>> `SESSION` is set in the PC's terminal, so this check runs there: `ssh`
>> with a command after the address runs that one command on the Orin and
>> prints its answer on the PC.
>>
>> > [!IMPORTANT]
>> > Run in the PC's terminal, from the repo root
>>
>> > [!CAUTION]
>> > Not copy/paste on another rig: replace `tony@192.168.0.215` with your
>> > Orin's user and address.
>>
>> ```pwsh
>> ssh tony@192.168.0.215 cat orin-cam/brain.out | Select-String "worker ready: policy $SESSION/", "brain idle at"
>> ```
>>
>>> #### What each part means
>>>
>>> - **`ssh tony@192.168.0.215 cat orin-cam/brain.out`**: prints the brain's
>>>   file from the Orin. `orin-cam/brain.out` has no `~/`: `ssh` starts in
>>>   the Orin's home folder, and PowerShell could turn a `~` into the PC's
>>>   own home folder before `ssh` sends it.
>>> - **`Select-String "…", "…"`**: keeps only the lines that hold either
>>>   text: the ready line for `${SESSION}`, and the brain's `brain idle at`.
>>
>>> #### Expected Results:
>>> Both lines:
>>>
>>> ```text
>>> worker ready: policy ${SESSION}/latest.zip, <steps> steps, commit <commit>, top rate 44600 pulses/s, 6 past commands, ages in units of 1000 ms
>>> brain idle at http://127.0.0.1:8091
>>> ```
>>>
>>> > [!WARNING]
>>> > Anything less means the robot is not running this session's policy
>>> > yet:
>>> >
>>> > - Only the `brain idle at` line: the brain loaded another session's
>>> >   policy, or `SESSION` is not set in this window (`$SESSION` prints an
>>> >   empty line).
>>> > - Only the `worker ready:` line: the brain stopped after loading; the
>>> >   `Error:` line that 7b shows says why.
>>> > - Nothing: the worker is still loading (wait the 60 s and run this
>>> >   again), or the brain stopped before it was ready (the `Error:` line
>>> >   that 7b shows).
>
> The brain now runs the new policy. Do not press Run yet:
> [03-RUN.md](03-RUN.md) takes it from here, with what has to be ready
> before a run.

## Keeping it

After [03-RUN.md](03-RUN.md) has shown the new policy balances, keep it.
`__live__/orin/python/saved/policy.npz` is in git. A new policy only ever
replaces it whole, with a fresh export, never by editing or merging it, so
committing the new file records exactly which policy the robot runs. This
commits that one file alone, with the session's name in its message:

> [!IMPORTANT]
> Run in the PC's terminal, from the repo root, with `SESSION` set as at the
> top

```pwsh
git commit -m "Deploy policy $SESSION" -- __live__/orin/python/saved/policy.npz
```

`-- <file>` commits only that file, whatever else has changed.

## Going back

If the new policy does worse than the one before, go back to it. Until the
new file is committed, git still holds the one before it. This puts the last
committed policy back on the PC:

> [!IMPORTANT]
> Run in the PC's terminal, from the repo root

```pwsh
git restore __live__/orin/python/saved/policy.npz
```

This gives the robot an older session's policy, so here `SESSION` must name
that older session, not the newest one. The file names the session it came
from, so set `SESSION` from the file itself:

> [!IMPORTANT]
> Run in the PC's terminal, from the repo root

```pwsh
$SESSION = uv run --project __synthetic__/trainer/python python -c "import numpy; print(str(numpy.load('__live__/orin/python/saved/policy.npz')['source']).split('/')[0])"
```

`numpy.load(…)['source']` and `.split('/')[0]` are explained in Step 2a;
`$SESSION = …` keeps the name the command printed. Then repeat Step 2b and
Steps 3 to 7 to give the policy to the robot.

`SESSION` now names the restored, older session in this window, not the
newest one. Set it again with the command at the top before following
01-TRAIN.md or this file's Step 1 in this window.

> [!WARNING]
> Skip Steps 1 and 2a here. Step 1 would overwrite the policy you just
> restored with the newest session's export, and with `SESSION` read from
> the file, 2a would only compare the file with itself. Step 2b still
> applies: run it to check the restored policy trained with the robot's
> detector.

An older one, from any earlier commit, comes back the same way with that
commit named. A commit's message does not say which session its policy came
from, so this lists every commit that changed the policy, newest first, each
with its short name (the first few characters of its id) and the `source`
line written in that commit's `policy.npz`:

> [!IMPORTANT]
> Run in the PC's terminal, from the repo root

```pwsh
uv run --project __synthetic__/trainer/python python -c "import io, subprocess, numpy; p = '__live__/orin/python/saved/policy.npz'; [print(c, numpy.load(io.BytesIO(subprocess.run(['git', 'show', c + ':' + p], capture_output=True, check=True).stdout))['source']) for c in subprocess.run(['git', 'log', '--format=%h', '--', p], capture_output=True, text=True, check=True).stdout.split()]"
```

> ### What each part means
>
> - **`git log --format=%h -- <file>`**: the short names of the commits that
>   changed the file, newest first.
> - **`git show <commit>:<file>`**: the file's bytes as that commit holds them,
>   without changing the file on disk.
> - **`numpy.load(io.BytesIO(…))['source']`**: reads those bytes as a policy
>   file and takes its `source` line, as Step 2a does.

> ### Expected Results:
> One line per commit. On 2026-11-14 there was one:
>
> ```text
> 6bb511d1 20261108_115227/latest.zip, 6,000,000 steps, commit 7a5e28c1 with uncommitted changes
> ```

`saved/` also holds `policy-20261101_163458.npz`, an older policy (session
`20261101_163458`, 4,000,008 steps) kept in git under its session's name.
Nothing loads it: the brain's worker loads only `saved/policy.npz`
(`POLICY_PATH` in `brain_worker.py`).

> [!CAUTION]
> Not copy/paste: replace `<commit>` with the short name of the commit that
> holds the policy you want, from the list above.

```pwsh
git restore --source <commit> __live__/orin/python/saved/policy.npz
```

Then set `SESSION` from the file and repeat Step 2b and Steps 3 to 7, as
above, skipping Steps 1 and 2a.
