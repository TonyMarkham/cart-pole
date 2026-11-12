# 01-TRAIN — teaching the policy to balance the pole

The robot balances the pole by moving the cart. The **policy** is the small
neural network that decides how to move it: it reads where the detector saw
the markers in the last six camera frames and answers with the cart's next
speed. It learns by trial and error, in a simulation of the rig, so the real
rig is never at risk while it learns. This file starts that learning, a
**training session**, and says where its results go.

One training session runs everything at once
([the diagram](../__documentation__/synthetic-loop-dia.md) has the sequence):

- the **trainer** (`trainer/python/`), which holds the policy and teaches it
  with SAC, a learning method made for one continuous action like the cart's
  speed;
- the **orchestrator** (`orchestrator/rust/`), which runs MuJoCo, the physics,
  and starts every other program;
- one **Unity player** per simulated robot: the camera, which draws the rig in
  each pose MuJoCo gives;
- one **synthetic Orin** per simulated robot (`orin/python/`): it sends its
  player the pose, gets the picture back, and finds the markers in it with the
  same detector the robot runs.

Every command runs from the repo root, in PowerShell.

> ## Step 1 — what has to exist first
>
> | What | Where | How it is made |
> |---|---|---|
> | The Unity player | `unity/projects/SyntheticCamera/Build/SyntheticCamera.exe` | [01-GENERATE.md](../__object_detection__/01-GENERATE.md), Step 1 |
> | The detector's TensorRT engine | the `--engine` path in `trainer/python/settings.toml` `[orchestrator] command` | [04-SYNTHETIC_ORIN.md](../__object_detection__/04-SYNTHETIC_ORIN.md) |
> | MuJoCo's library | `mujoco_dir` in `orchestrator/rust/orchestrator.toml` | the greyhound checkout beside this repo |
>
> The player is the same program that makes the detector's training frames.
> Here it runs as the camera: the orchestrator starts it with
> `-player-mode SyntheticCamera -state-server <port>`, and the player waits on
> that port for its Orin to connect.

> ## Step 2 — build the orchestrator
>
>> ### PowerShell
>> ```pwsh
>> cargo build --release -p orchestrator
>> ```
>
> This writes `target/release/orchestrator.exe`, the program the trainer
> starts. Build it again after any change under `orchestrator/rust/`.

> ## Step 3 — check the parts that need no player
>
>> ### PowerShell
>> ```pwsh
>> cargo test -p orchestrator
>> uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/test_calibration.py
>> uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/test_derived_state.py
>> ```
>
> On 2026-11-12 these passed 38, 11 and 11 tests.

> ## Step 4 — check one simulated robot, for one episode
>
>> ### PowerShell
>> ```pwsh
>> uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/env_check.py 1
>> ```
>
> One **episode** is one try at balancing, from the pole hanging at rest
> until the cart hits a limit switch or 500 steps have passed. This runs the
> whole loop with one player and one Orin, in this order:
>
> 1. **The calibration.** The detector gives pixels and the policy needs
>    metres, so a player and an Orin of their own render the rig in seven
>    still poses and fit the matrix that turns one into the other. Each pose
>    prints a line such as `calibration: left horizontal: 30 images from 30
>    renders`, and the fit `calibration: worst point 0.157 px` (2026-11-12);
>    over `[calibration] residual_px` in `orin/python/settings.toml`, 1.0 px,
>    the run stops here. Both programs then quit.
> 2. **The episode.** A fresh player and Orin start, the Orin prints
>    `connected to the player at ws://127.0.0.1:9001/`, and the episode runs,
>    one line per step.
>
> It ends with `env_check: episode 0: … steps, ended by …` and exit code 0.
> Its files go to `target/trainer-run/`.

> ## Step 5 — train
>
>> ### PowerShell
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
> `[train] save_every` (10,000). Each finished episode prints one line:
>
> ```text
> train: episode    5  instance 5  step   2,976/3,000   248 steps  fault             reward    23.55  mean 0.0949  elapsed 0h01m  left 0h00m
> ```
>
> Step 4 of `__synthetic__/UPDATE.md` ran this command on 2026-11-12 with
> `--settings` naming a copy of `settings.toml` whose `gym` was a copy of
> `gym.toml` with `total_steps = 3_000` and `save_every = 1_000`: 12 robots,
> saves at steps 1,008, 2,004 and 3,000, every player and Orin exiting with
> code 0.
>
> A session that stops, by failing or by Ctrl+C, continues from its last save
> with `--resume` (`train.py`'s own header says how).

> ## Where the results go
>
> Each session gets its own folder, `trainer/python/runs/<date>_<time>/`
> (`[train] runs_dir`, not in git):
>
> | File | What it holds |
> |---|---|
> | `latest.zip`, `latest_buffer.pkl`, `latest.json` | the newest save: the policy, the steps it learns from, and what the session was run with |
> | `model_<steps>.zip` / `.json` | the policy at each save, kept |
> | `episodes.csv` | one row per finished episode |
> | `loop.log` | everything the orchestrator, the players and the Orins printed |
> | `loop_from_<steps>/` | each robot's frames and logs, and the calibration's `homography.json` |
> | `settings_from_<steps>.toml`, `gym_from_<steps>.toml` | the settings each start of the session used |
>
> To give the real robot a trained policy, see `__synthetic__/README.md`,
> *To write a trained policy for the real robot*.
