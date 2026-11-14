# 01-TRAIN — teaching the policy to balance the pole

The robot balances the pole by moving the cart. The **policy** is the small
neural network that decides how to move it: it reads where the detector saw
the markers in the last six camera frames, and its own last six commands, and
answers with the cart's next speed. It learns by trial and error, in a
simulation of the rig, so the real rig is never at risk while it learns. This
file builds and checks the simulation and trains the policy, and ends with a
trained **training session**: one folder in `trainer/python/runs/` holding the
policy. The three numbered files go in order, each starting where the one
before ends:

1. **01-TRAIN.md**, this file: a trained session on the PC.
2. **[02-DEPLOY.md](02-DEPLOY.md)**: that session's policy written into the
   file the robot runs, checked, and given to the robot.
3. **[03-RUN.md](03-RUN.md)**: one balance run on the real rig with it.

The trainer's own code calls the folder a *run* and each start of it, the
first and each resume, a *session*. These files call the whole folder a
training session, to keep *run* for a balance run on the rig.

One training session runs all of these at once
([the diagram](../__documentation__/synthetic-loop-dia.md) has the sequence):

- the **trainer**, which holds the policy and teaches it with SAC, a learning
  method made for one continuous action like the cart's speed;
- the **orchestrator**, which runs MuJoCo, the physics simulator, steps the
  simulated rig, and starts every other program;
- one **Unity player** per simulated robot: the camera, which draws the rig in
  each pose MuJoCo gives;
- one **synthetic Orin** per simulated robot, standing in for the robot's
  computer: it sends its player the pose, gets the picture back, and finds the
  markers in it with the same detector the robot runs.

Every command runs in the PC's terminal, PowerShell, from the repo root.

## What is in `__synthetic__/` for training

```text
__synthetic__/
├── 01-TRAIN.md                   this file
├── 02-DEPLOY.md                  the next: giving the trained policy to the robot
├── 03-RUN.md                     the last: one balance run on the rig
├── orchestrator/rust/
│   ├── orchestrator.toml         where MuJoCo's library is, read when building
│   └── crates/
│       ├── orchestrator/         drives the loop, starts the players and the Orins
│       ├── physics/              MuJoCo, inside the orchestrator
│       └── arduino/              the synthetic Arduino: the stepper's pulse rate, its
│                                 ramp and the rail's limit switches, inside the orchestrator
├── orin/python/                  the synthetic Orin, a uv project
│   └── settings.toml             the detector, the connection to the player, the calibration
└── trainer/python/               the trainer, a uv project, which starts every training session
    ├── settings.toml             the session's setup: ports, robots, screen, timings, the
    │                             MJCF, the orchestrator's command line
    ├── gym.toml                  what the policy sees, does and is rewarded for, and SAC's values
    ├── measures.toml             the values of the measures run on finished sessions
    │                             (measure.py), and 02-DEPLOY.md's export's ([export_policy])
    └── runs/                     one folder per training session (not in git)
```

The **MJCF** is MuJoCo's model of the rig: its parts, masses, joints and
motor, in one XML file, exported from the rig's CAD by the Unity project
(`[mjcf]` in `trainer/python/settings.toml` names it).

A **uv project** is a Python folder with a `pyproject.toml`; `uv run --project`
runs its code with the packages it lists, installed in its own `.venv/`. A
**crate** is one Rust package; the crates are in the workspace of the repo
root's `Cargo.toml`, so what they build goes to the root's `target/`.

> ## SESSION — the training session's name
>
> Each training session gets its own folder, `trainer/python/runs/<date>_<time>/`,
> named for the moment Step 5 started it: `20261113_074052` is 13 November 2026
> at 07:40:52. Resuming a stopped session (Step 5) names that folder with one
> shell variable, `SESSION`, and [02-DEPLOY.md](02-DEPLOY.md) sets the same
> variable with the same command. A resume needs a save to start from, so
> `SESSION` is the newest session that has one. A variable lives only in the
> PowerShell window it was set in, so set it in the window that runs the
> resume:
>
>> ### PowerShell
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> $SESSION = (Get-ChildItem __synthetic__/trainer/python/runs -Directory | Where-Object { Test-Path (Join-Path $_.FullName "latest.json") } | Sort-Object Name | Select-Object -Last 1).Name
>> ```
>
>> ### What each part means
>>
>> - **`Get-ChildItem … -Directory`**: the folders in `runs/`, one per session.
>> - **`Where-Object { Test-Path … "latest.json" }`**: keeps only the folders
>>   that hold a save, `latest.json` with `latest.zip`. A session gets its
>>   folder the moment it starts, and saves every `[train] save_every` steps
>>   (`gym.toml`, 10,000), and again whenever it ends: at its total, on Ctrl+C,
>>   or on a failure (`train.py`, its `finally`). Only a session that failed
>>   before training began has no save.
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
>> The newest session's name: on 2026-11-14, `20261113_074052`, the session
>> that ran to 6,000,000 steps. An empty line means no session in `runs/` has
>> a save yet.
>>
>> > [!WARNING]
>> > The newest session with a save is not always the one you mean: a short
>> > test session started later, and stopped, has a save too, and becomes
>> > `SESSION`. Check its name before resuming it.

> ## Step 1 — what has to exist first
>
> | What | Where | How it is made |
> |---|---|---|
> | The Unity player | `unity/projects/SyntheticCamera/Build-1.1m/SyntheticCamera.exe`, the `--player` path in `trainer/python/settings.toml` `[orchestrator] command` | [01-GENERATE.md](../__object_detection__/01-GENERATE.md), Step 1, choosing the folder `Build-1.1m/` instead of `Build/` |
> | The detector's TensorRT engine | the `--engine` path in `trainer/python/settings.toml` `[orchestrator] command` | [04-SYNTHETIC_ORIN.md](../__object_detection__/04-SYNTHETIC_ORIN.md): on a fresh clone only its *WEIGHTS* section and Step 2, since the settings already name the detector `640-21k` |
> | MuJoCo's library | `mujoco_dir` in `orchestrator/rust/orchestrator.toml` | the greyhound checkout beside this repo |
>
> The player is the same program that makes the detector's training frames.
> `Build-1.1m` is a build of the scene with its camera 1.1 m up, where commit
> `747d9d0f` moved it from 0.9 m for the policy's training; the scene in git
> has had the camera there since. Here it runs as the camera: the
> orchestrator starts it with
> `-player-mode SyntheticCamera -state-server <port>`, and the player waits on
> that port for its Orin to connect.
>
> The **TensorRT engine** is the trained detector compiled by TensorRT,
> NVIDIA's software for running a model fast on one graphics card; this one
> is built for this PC's card. **greyhound** (`greyhound-snake`) is the
> author's sibling repository, which provides the STEP import, the MJCF
> export and MuJoCo's library; it is not in this repo, and this repo expects
> it checked out beside it (the repo root's `AGENTS.md`, *How greyhound
> content is consumed*).

> ## Step 2 — build the orchestrator
>
>> ### PowerShell
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> cargo build --release -p orchestrator
>> ```
>
> This writes `target/release/orchestrator.exe`, the program the trainer
> starts. Build it again after any change under `orchestrator/rust/`.

> ## Step 3 — check the parts that need no player
>
> Three separate checks. None of them starts Unity or uses the GPU, so each
> takes seconds. Each ends by saying how many tests passed; a failure names
> the test that failed.
>
>> ### 3a. The orchestrator
>>
>> Checks the orchestrator's own rules: among others, that the same seed
>> always makes the same schedule of timings, the grid the players' windows
>> are laid out in, and that a closed socket closes cleanly.
>>
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> cargo test -p orchestrator
>> ```
>>
>> A pass ends with `test result: ok. 38 passed; 0 failed`.
>
>> ### 3b. The calibration's arithmetic
>>
>> Checks the maths that turns the detector's pixels into metres, on points
>> whose answers are known: how a marker's many detections become one point,
>> the lens correction, the fit of the matrix, and the saved matrix being read
>> back.
>>
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/test_calibration.py
>> ```
>>
>> A pass ends with `Ran 11 tests`, then `OK`.
>
>> ### 3c. The five numbers the Orin works out
>>
>> Checks how the Orin works out, from the markers it saw, the cart's place
>> and speed and the pole's angle and speed, including when a marker was not
>> seen, on poses whose answers can be worked by hand.
>>
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/test_derived_state.py
>> ```
>>
>> A pass ends with `Ran 11 tests`, then `OK`.

> ## Step 4 — check one simulated robot, for one episode
>
>> ### PowerShell
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/env_check.py 1
>> ```
>
> One **episode** is one try at balancing, from the pole hanging at rest
> until the cart reaches a **limit switch**, one of the two switches at the
> rail's ends that the cart trips, or 500 steps have passed. This runs the
> whole loop with one player and one Orin, with random commands, in this order:
>
> 1. **The calibration.** The detector gives pixels and the policy needs
>    metres, so a player and an Orin of their own render the rig in seven
>    still poses and fit the matrix that turns one into the other, from
>    MuJoCo's true positions of the markers. Each pose prints a line such as
>    `calibration: left horizontal: 30 images from 30 renders`, and the fit
>    `calibration: worst point 0.157 px`. A point further off
>    than `[calibration] residual_px` in `orin/python/settings.toml`, 1.0 px,
>    stops the check here, and so does a pose whose markers are not seen. Both
>    programs then quit.
> 2. **The episode.** A fresh player and Orin start, the Orin prints
>    `connected to the player at ws://127.0.0.1:9001/`, and the episode runs,
>    one line per step.
>
> It ends with `env_check: episode 0: … steps, ended by …` and exit code 0.
> Its files go to `target/trainer-run/`: the calibration's `homography.json`,
> and in `instance_0/` the frames, `observations.jsonl` (what the Orin saw)
> and `states.csv` (what MuJoCo knew).

> ## Step 5 — train
>
>> ### PowerShell
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/train.py
>> ```
>
> This runs the calibration as in Step 4, then `instance_rows` ×
> `instance_columns` simulated robots at once (`trainer/python/settings.toml`
> `[orchestrator]`, 3 × 4 = 12 today), each with its own player, whose windows
> fill the screen as a grid. Robot k's player listens on port
> `unity_port + 2k`, 9001 to 9023 for 12.
>
> The session takes `[train] total_steps` steps, every robot's counting
> (`trainer/python/gym.toml`, 6,000,000 today), and saves every
> `[train] save_every` (10,000). Each finished episode prints one line, such
> as this one, from a short session of 3,000 steps (a full session shows
> `/6,000,000`):
>
> ```text
> train: episode    5  instance 5  step   2,976/3,000   248 steps  fault             reward    23.55  mean 0.0949  elapsed 0h01m  left 0h00m
> ```
>
> Read left to right: the episode's number; which robot ran it; the session's
> steps so far, of its total; how many steps the episode lasted; how it ended
> (`fault`: the cart reached a limit switch; `time limit`: 500 steps passed;
> or the name of another end rule in `gym.toml`); the episode's total reward
> and its reward per step; and the session's time so far and the time it
> expects is left.
>
> A session that stops, by failing or by Ctrl+C, continues from its last
> save. Set `SESSION` first (*SESSION — the training session's name*, at the
> top), then:
>
>> ### PowerShell
>> > [!IMPORTANT]
>> > Run from the repo root: the folder with `Cargo.toml` and `AGENTS.md` in it.
>>
>> ```pwsh
>> uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/train.py --resume "__synthetic__/trainer/python/runs/$SESSION"
>> ```
>
> The session continues in the same folder, and refuses to start, with its
> reason, if what the policy sees, how an episode ends, or the values the
> simulation reports about the rig (`run_values` in `latest.json`: the top
> pulse rate, the pole's mass, and the like) changed since the session began
> (`train.py`'s own header).
>
> Each session gets its own folder, `trainer/python/runs/<date>_<time>/`
> (`[train] runs_dir`, not in git):
>
> | File | What it holds |
> |---|---|
> | `latest.zip`, `latest_buffer.pkl`, `latest.json` | the newest save: the policy, the steps it learns from, and what the session was run with |
> | `model_<steps>.zip` / `.json` | the policy at each save, kept |
> | `episodes.csv` | one row per finished episode |
> | `captures_from_<steps>.csv` | one row per capture from each start of the session: what the policy saw and did, MuJoCo's true positions, and the reward's parts |
> | `loop.log` | everything the orchestrator, the players and the Orins printed |
> | `loop_from_<steps>/` | each robot's frames and logs, and the calibration's `homography.json` |
> | `settings_from_<steps>.toml`, `gym_from_<steps>.toml` | the settings each start of the session used |

## Where this file ends

Step 5 ends by itself once the session has taken `[train] total_steps`, and
its last save is the trained policy: `latest.zip` in its folder. Training
does not touch the file the robot runs,
`__live__/orin/python/saved/policy.npz`. [02-DEPLOY.md](02-DEPLOY.md) takes
it from here: it writes the
session's policy into that file, checks it, and gives it to the robot.
