"""The training run: SAC learns the policy on the loop (trainer-plan.md T9).

The policy has to learn from the loop itself, episode after episode, for
hours, so this runs Stable-Baselines3's SAC on the group of training
instances (settings.toml's [orchestrator] instances, vec_env.py) until it
has taken gym.toml's [train] total_steps, every instance's steps counting
toward it, and keeps the run recoverable: every [train] save_every steps it
saves the model and its replay buffer, so a run that fails or is stopped
(Ctrl+C) resumes from its last save instead of starting over.

Each training run gets a folder, gym.toml's [train] runs_dir /
<date>_<time>/, holding:

- settings.toml and gym.toml as they were at the start, so the run's values
  are known later;
- latest.zip and latest_buffer.pkl, the model and replay buffer at the last
  save (each written beside, then swapped in, so a crash mid-save keeps the
  previous one);
- model_<steps>.zip, a model snapshot at every save;
- loop.log, the run's own output (the orchestrator's line per observation
  of each instance, the Unity players' and the Orins'), so the console holds
  only one line per episode and one per save;
- episodes.csv, one row per finished episode, in the order they finish: its
  number, the instance it ran on, its seed, the run's steps so far, its
  length, how it ended (time limit or end crash), its total reward, and the
  pole part of the reward (0 hanging, 1 upright, reward.py) at its highest
  and on average.

Episode e runs with seed settings.toml's seed + e (vec_env.py), so the
latency draws differ between episodes and the run repeats.

A resumed run continues the same folder: the model and buffer from its last
save, with the SAC settings it was saved with; [train] total_steps and
save_every are read again, so a run can be extended by raising total_steps.
Its instances must be as many as when it was saved, since the replay buffer
holds one column per instance. Episodes still running at the last save were
never logged, and numbering carries on after the highest logged.

usage (from the repo root):
  uv run --project trainer python trainer/src/train.py [--settings FILE] [--resume [RUN_FOLDER]]
--resume alone continues the most recent run in [train] runs_dir that has
a save.
"""

import argparse
import csv
import os
import shutil
import sys
import time
from datetime import datetime
from pathlib import Path

from stable_baselines3 import SAC
from stable_baselines3.common.callbacks import BaseCallback

from gym_settings import GymSettings
from launch import RunFailed
from settings import DEFAULT_PATH, Settings
from train_settings import TrainSettings
from vec_env import LoopVecEnv

LATEST = Path("<latest>")  # --resume with no folder

COLUMNS = ["episode", "instance", "seed", "steps", "length", "ended", "reward", "pole_max", "pole_mean", "wall_s"]


def save(model: SAC, folder: Path, snapshot: bool) -> None:
    latest, buffer = folder / "latest.zip", folder / "latest_buffer.pkl"
    model.save(folder / "latest.saving.zip")
    model.save_replay_buffer(folder / "latest_buffer.saving.pkl")
    os.replace(folder / "latest.saving.zip", latest)
    os.replace(folder / "latest_buffer.saving.pkl", buffer)
    if snapshot:
        shutil.copyfile(latest, folder / f"model_{model.num_timesteps}.zip")
    print(f"train: saved at step {model.num_timesteps} -> {folder}", flush=True)


class Progress(BaseCallback):
    """Logs each finished episode and saves every save_every steps."""

    def __init__(self, folder: Path, save_every: int, total_steps: int, instances: int, first_step: int) -> None:
        super().__init__()
        self.folder = folder
        self.save_every = save_every
        self.total_steps = total_steps
        self.first_step = first_step  # where this session began, for its time left
        self.session_started = time.perf_counter()
        # The instances step together, so the step count moves by their
        # number: a save is due each time it crosses a multiple of save_every.
        self.saved_block = first_step // save_every
        self.saved_at = None  # the step of the last save, so the run's end does not repeat it
        self.file = open(folder / "episodes.csv", "a", newline="", encoding="utf-8")
        self.rows = csv.writer(self.file)
        if self.file.tell() == 0:
            self.rows.writerow(COLUMNS)
        self.length = [0] * instances
        self.reward = [0.0] * instances
        self.pole_max = [0.0] * instances
        self.pole_sum = [0.0] * instances
        self.started = [time.perf_counter()] * instances

    def _on_step(self) -> bool:
        for k, info in enumerate(self.locals["infos"]):
            self.length[k] += 1
            self.reward[k] += float(self.locals["rewards"][k])
            self.pole_max[k] = max(self.pole_max[k], info["pole"])
            self.pole_sum[k] += info["pole"]
            if self.locals["dones"][k]:
                self.episode_ended(k, info)
        block = self.num_timesteps // self.save_every
        if block > self.saved_block:
            save(self.model, self.folder, snapshot=True)
            self.saved_block, self.saved_at = block, self.num_timesteps
        return True

    def episode_ended(self, k: int, info: dict) -> None:
        ended = "end crash" if info["crashed"] else "time limit"
        wall = time.perf_counter() - self.started[k]
        mean = self.pole_sum[k] / self.length[k]
        self.rows.writerow([
            info["episode_number"], k, info["seed"], self.num_timesteps, self.length[k], ended,
            f"{self.reward[k]:.3f}", f"{self.pole_max[k]:.4f}", f"{mean:.4f}", f"{wall:.1f}",
        ])
        self.file.flush()
        elapsed = time.perf_counter() - self.session_started
        done = max(self.num_timesteps - self.first_step, 1)
        left = elapsed / done * max(self.total_steps - self.num_timesteps, 0)
        print(f"train: episode {info['episode_number']:4d}  instance {k}  "
              f"step {self.num_timesteps:7,d}/{self.total_steps:,d}  "
              f"{self.length[k]:4d} steps  {ended:<10}  reward {self.reward[k]:7.1f}  "
              f"pole max {self.pole_max[k]:.3f} mean {mean:.3f}  "
              f"elapsed {clock(elapsed)}  left {clock(left)}", flush=True)
        self.length[k], self.reward[k], self.pole_max[k], self.pole_sum[k] = 0, 0.0, 0.0, 0.0
        self.started[k] = time.perf_counter()

    def _on_training_end(self) -> None:
        self.file.close()


def clock(seconds: float) -> str:
    minutes = round(seconds / 60)
    return f"{minutes // 60}h{minutes % 60:02d}m"


def latest_run(runs_dir: Path) -> Path:
    """The most recent run folder with a save. Folders are named
    <date>_<time>, so the names sort in time order."""
    saved = sorted(f for f in runs_dir.iterdir() if (f / "latest.zip").is_file()) if runs_dir.is_dir() else []
    if not saved:
        raise SystemExit(f"train: no run in {runs_dir} has a save to resume from")
    return saved[-1]


def next_episode(folder: Path) -> int:
    """The number after the highest episode episodes.csv logged, 0 if none."""
    path = folder / "episodes.csv"
    if not path.is_file():
        return 0
    with open(path, newline="", encoding="utf-8") as f:
        numbers = [int(row["episode"]) for row in csv.DictReader(f)]
    return max(numbers) + 1 if numbers else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--settings", type=Path, default=DEFAULT_PATH,
                        help="the trainer's settings.toml (default: trainer/settings.toml)")
    parser.add_argument("--resume", type=Path, nargs="?", const=LATEST,
                        help="a run folder to continue from its last save; alone, the most recent run")
    args = parser.parse_args()

    settings = Settings.load(args.settings.resolve())
    gym_settings = GymSettings.load(settings.gym)
    train = TrainSettings.load(settings.gym)

    if args.resume is not None:
        folder = latest_run(train.train.runs_dir) if args.resume == LATEST else args.resume.resolve()
        if not (folder / "latest.zip").is_file():
            raise SystemExit(f"train: {folder} has no latest.zip to resume from")
    else:
        folder = train.train.runs_dir / datetime.now().strftime("%Y%m%d_%H%M%S")
        folder.mkdir(parents=True)
        shutil.copyfile(args.settings, folder / "settings.toml")
        shutil.copyfile(settings.gym, folder / "gym.toml")
    episode = next_episode(folder)
    env = LoopVecEnv(settings, gym_settings, log=folder / "loop.log", first_episode=episode)

    if args.resume is not None:
        model = SAC.load(folder / "latest.zip", env=env, device=train.sac.device)
        model.load_replay_buffer(folder / "latest_buffer.pkl")
        if model.replay_buffer.n_envs != env.num_envs:
            raise SystemExit(
                f"train: {folder} was saved with {model.replay_buffer.n_envs} instance(s), and settings.toml's "
                f"instances is {env.num_envs}: its replay buffer holds one column per instance, so resume it "
                f"with instances = {model.replay_buffer.n_envs}, or start a new run"
            )
        print(f"train: resuming {folder} at step {model.num_timesteps}, episode {episode}, "
              f"{env.num_envs} instance(s)", flush=True)
    else:
        # SB3 seeds NumPy and PyTorch, which take a 32-bit seed.
        model = SAC("MlpPolicy", env, seed=settings.seed % 2**32, verbose=0, **train.sac.arguments())
        print(f"train: new run {folder}, {env.num_envs} instance(s)", flush=True)

    remaining = train.train.total_steps - model.num_timesteps
    if remaining <= 0:
        print(f"train: already at step {model.num_timesteps} of {train.train.total_steps}", flush=True)
        return 0
    progress = Progress(folder, train.train.save_every, train.train.total_steps, env.num_envs, model.num_timesteps)
    code = 0
    try:
        model.learn(total_timesteps=remaining, callback=progress, reset_num_timesteps=args.resume is None)
    except KeyboardInterrupt:
        print("train: stopped (Ctrl+C)", flush=True)
        code = 130
    except RunFailed as error:
        print(f"train: a run failed, training ends: {error}", flush=True)
        code = 1
    finally:
        if progress.saved_at != model.num_timesteps:
            save(model, folder, snapshot=True)
        try:
            env.close()
        except Exception as error:  # the run may already be gone after Ctrl+C or a failure
            print(f"train: closing the environment: {error}", file=sys.stderr, flush=True)
    if code != 0:
        print(f"train: resume with --resume {folder}", flush=True)
    return code


if __name__ == "__main__":
    sys.exit(main())
