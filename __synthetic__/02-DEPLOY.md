# 02-DEPLOY — giving the robot the new policy

The robot balances the pole with the policy: the small network that reads
where the markers were in the last six frames, and its own last six commands,
and picks the cart's next speed. Training a new policy changes nothing on the
robot: its computer, a Jetson **Orin** Nano Super, keeps running the old one
until it is given the new one. This file checks the new policy, copies it to
the Orin, checks it again there, and restarts the one program that uses it,
the **brain**, which runs the balance loop.

The policy these steps use is `__live__/orin/python/saved/policy.npz` on the
PC, the file [01-TRAIN.md](01-TRAIN.md)'s Step 6 wrote from a training
session. It is one numpy file: the network's numbers, the values it was
trained with, and 1,000 of the session's own inputs with the trainer's answer
to each, so any copy of it can be checked against training.

## Two terminals

The steps run on two machines, the PC and the Orin, each in its own terminal.
Open both now and keep both open until the last step. Every step names the
terminal it runs in.

> ### The PC's terminal
>
> Open PowerShell and go to the repo root, the folder with `Cargo.toml` and
> `AGENTS.md` in it:
>
> ```pwsh
> cd D:/git/cart-pole
> ```

> ### The Orin's terminal
>
> Open a second PowerShell and log in to the Orin with **`ssh`**, which gives
> a terminal on another computer over the network. The Orin is `robo-orin`,
> at `192.168.0.215`, user `tony`:
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
│   ├── saved/policy.npz        the policy the brain runs, replaced by Step 2
│   └── src/test_policy.py      the policy's check, run by Step 3
└── orin-cam/                   the webserver and the brain
    └── target/release/brain    the brain, restarted by Steps 4 and 5
```

Only the brain loads the policy, and only when it starts, through its Python
worker, `~/live-orin-python/src/brain_worker.py`. So the brain alone is
restarted; the calibration service never reads the policy.

> ## Step 1 — check the policy on the PC
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
>> Five of the tests check the arithmetic on a network small enough to work
>> out by hand; the sixth checks `saved/policy.npz` itself against the
>> trainer's answers.

> ## Step 2 — copy the policy to the Orin (not run yet)
>
> This packs `__live__/orin/python` into one file, copies it over and unpacks
> it into `~/live-orin-python`, as
> [03-REAL_ORIN.md](../__object_detection__/03-REAL_ORIN.md)'s Step 3 does for
> the detector's settings. Files of the same name are replaced and nothing is
> deleted, so what only the Orin has (its lens and calibration results)
> stays.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root
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
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root
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
> Then check the copy is whole: a **SHA-256 checksum** is a 64-character
> fingerprint of a file's bytes, and two files with the same one are the same
> file.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> Get-FileHash -Algorithm SHA256 __live__/orin/python/saved/policy.npz
>> ```
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> sha256sum ~/live-orin-python/saved/policy.npz
>> ```
>
>> ### Expected Results:
>> The same 64 characters from both. PowerShell prints them in capitals and
>> `sha256sum` in small letters; that is the only difference.

> ## Step 3 — check the policy on the Orin (not run yet)
>
> The same check as Step 1, now with the Orin's own numpy, on the copy the
> brain will load.
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> cd ~/live-orin-python && .venv/bin/python src/test_policy.py
>> ```
>
>> ### Expected Results:
>> As in Step 1: `Ran 6 tests`, then `OK`.

> ## Step 4 — stop the brain (not run yet)
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

> ## Step 5 — start the brain (not run yet)
>
> It keeps running after the SSH session ends: `setsid` gives it a session of
> its own, `nohup` keeps it running when this one closes, `< /dev/null` gives
> it no keyboard, and `> … 2>&1` sends everything it prints into a file. It
> does not start by itself after the Orin reboots.
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> (
>>   cd ~/orin-cam &&
>>   nohup setsid ./target/release/brain > brain.out 2>&1 < /dev/null &
>> )
>> ```
>
> > [!IMPORTANT]
> > Run in the Orin's terminal, from `~`
>
>> ### bash (the Orin's terminal)
>> ```bash
>> cat ~/orin-cam/brain.out
>> ```
>
>> ### Expected Results:
>> After a few seconds, a `worker ready:` line naming the new policy's
>> training session, its save and its step count, then the brain waiting for
>> a run:
>>
>> ```text
>> worker ready: policy <session>/latest.zip, <steps> steps, ..., top rate 44600 pulses/s, 6 past commands, ages in units of 1000 ms
>> brain idle at http://127.0.0.1:8091
>> ```
>>
>> `<session>` is the training session's folder name, the `<date>_<time>`
>> 01-TRAIN.md's Step 6 exported from. A line naming the old session means
>> the brain loaded the old file: Step 2 did not reach the Orin. A
>> `[Worker Error]` line instead means the worker could not start; its own
>> message is above it in the file.
>
> The robot now balances with the new policy: start a run from
> <http://192.168.0.215:8080/run/>.

## Keeping it

`__live__/orin/python/saved/policy.npz` is in git. A new policy only ever
replaces it whole, with a fresh export, never by editing or merging it, so
committing the new file records exactly which policy the robot runs.

## Going back

Until the new file is committed, git still holds the one before it. This puts
the last committed policy back on the PC:

> [!IMPORTANT]
> Run in the PC's terminal, from the repo root

```pwsh
git restore __live__/orin/python/saved/policy.npz
```

Then repeat Steps 1 to 5 to give it to the robot. An older one, from any
earlier commit, comes back the same way with that commit named:
`git restore --source <commit> __live__/orin/python/saved/policy.npz`.
