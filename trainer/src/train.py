"""The training run: SAC learns the policy on the loop (trainer-plan.md T9).

The policy has to learn from the loop itself, episode after episode, for
hours, so this runs Stable-Baselines3's SAC on the environment until it has
taken gym.toml's [train] total_steps, and keeps the run recoverable: every
[train] save_every steps it saves the model and its replay buffer, so a run
that fails or is stopped (Ctrl+C) resumes from its last save instead of
starting over.

Each training run gets a folder, gym.toml's [train] runs_dir /
<date>_<time>/, holding:

- settings.toml and gym.toml as they were at the start, so the run's values
  are known later;
- latest.zip and latest_buffer.pkl, the model and replay buffer at the last
  save (each written beside, then swapped in, so a crash mid-save keeps the
  previous one);
- model_<steps>.zip, a model snapshot at every save;
- episodes.csv, one row per finished episode: its seed, the run's steps so
  far, its length, how it ended (time limit or end crash), its total reward,
  and the pole part of the reward (0 hanging, 1 upright, reward.py) at its
  highest and on average.

Each episode runs with its own seed, settings.toml's seed + the episode's
number, so the latency draws differ between episodes and the run repeats.

A resumed run continues the same folder: the model and buffer from its last
save, with the SAC settings it was saved with; [train] total_steps and
save_every are read again, so a run can be extended by raising total_steps.

usage (from the repo root):
  uv run --project trainer python trainer/src/train.py [--settings FILE] [--resume RUN_FOLDER]
"""

import argparse
import csv
import os
import shutil
import sys
import time
from datetime import datetime
from pathlib import Path

import gymnasium as gym
from stable_baselines3 import SAC
from stable_baselines3.common.callbacks import BaseCallback

from gym_settings import GymSettings
from launch import RunFailed
from loop_env import make
from settings import DEFAULT_PATH, SEED_LIMIT, Settings
from train_settings import TrainSettings

COLUMNS = ["episode", "seed", "steps", "length", "ended", "reward", "pole_max", "pole_mean", "wall_s"]


class SeedEachEpisode(gym.Wrapper):
    """Episode k runs with seed base + k; SB3 resets without a seed, which
    would repeat settings.toml's seed, and its latency draws, every episode."""

    def __init__(self, env: gym.Env, base: int, first_episode: int) -> None:
        super().__init__(env)
        self.base = base
        self.episode = first_episode - 1
        self.seed = None

    def reset(self, *, seed=None, options=None):
        self.episode += 1
        self.seed = (self.base + self.episode) % SEED_LIMIT
        return self.env.reset(seed=self.seed, options=options)

    def step(self, action):
        observation, reward, terminated, truncated, info = self.env.step(action)
        # The vector env resets as soon as an episode ends, so the seed is
        # read here, before the next episode's reset replaces it.
        info["seed"] = self.seed
        return observation, reward, terminated, truncated, info


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

    def __init__(self, folder: Path, save_every: int, total_steps: int, first_episode: int) -> None:
        super().__init__()
        self.folder = folder
        self.save_every = save_every
        self.total_steps = total_steps
        self.episode = first_episode
        self.saved_at = None  # the step of the last save, so the run's end does not repeat it
        self.file = open(folder / "episodes.csv", "a", newline="", encoding="utf-8")
        self.rows = csv.writer(self.file)
        if self.file.tell() == 0:
            self.rows.writerow(COLUMNS)
        self.start_episode()

    def start_episode(self) -> None:
        self.length, self.reward, self.pole_max, self.pole_sum = 0, 0.0, 0.0, 0.0
        self.started = time.perf_counter()

    def _on_step(self) -> bool:
        info = self.locals["infos"][0]
        self.length += 1
        self.reward += float(self.locals["rewards"][0])
        self.pole_max = max(self.pole_max, info["pole"])
        self.pole_sum += info["pole"]
        if self.locals["dones"][0]:
            ended = "end crash" if info["crashed"] else "time limit"
            wall = time.perf_counter() - self.started
            self.rows.writerow([
                self.episode, info["seed"], self.num_timesteps, self.length, ended,
                f"{self.reward:.3f}", f"{self.pole_max:.4f}", f"{self.pole_sum / self.length:.4f}", f"{wall:.1f}",
            ])
            self.file.flush()
            print(f"train: episode {self.episode}, step {self.num_timesteps}/{self.total_steps}: "
                  f"{self.length} steps, {ended}, reward {self.reward:.1f}, "
                  f"pole max {self.pole_max:.3f}, mean {self.pole_sum / self.length:.3f}", flush=True)
            self.episode += 1
            self.start_episode()
        if self.num_timesteps % self.save_every == 0:
            save(self.model, self.folder, snapshot=True)
            self.saved_at = self.num_timesteps
        return True

    def _on_training_end(self) -> None:
        self.file.close()


def finished_episodes(folder: Path) -> int:
    path = folder / "episodes.csv"
    if not path.is_file():
        return 0
    with open(path, newline="", encoding="utf-8") as f:
        return max(sum(1 for _ in f) - 1, 0)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--settings", type=Path, default=DEFAULT_PATH,
                        help="the trainer's settings.toml (default: trainer/settings.toml)")
    parser.add_argument("--resume", type=Path, help="a run folder to continue from its last save")
    args = parser.parse_args()

    settings = Settings.load(args.settings.resolve())
    gym_settings = GymSettings.load(settings.gym)
    train = TrainSettings.load(settings.gym)

    if args.resume is not None:
        folder = args.resume.resolve()
        if not (folder / "latest.zip").is_file():
            raise SystemExit(f"train: {folder} has no latest.zip to resume from")
    else:
        folder = train.train.runs_dir / datetime.now().strftime("%Y%m%d_%H%M%S")
        folder.mkdir(parents=True)
        shutil.copyfile(args.settings, folder / "settings.toml")
        shutil.copyfile(settings.gym, folder / "gym.toml")
    episode = finished_episodes(folder)
    env = SeedEachEpisode(make(settings, gym_settings), settings.seed, episode)

    if args.resume is not None:
        model = SAC.load(folder / "latest.zip", env=env, device=train.sac.device)
        model.load_replay_buffer(folder / "latest_buffer.pkl")
        print(f"train: resuming {folder} at step {model.num_timesteps}, episode {episode}", flush=True)
    else:
        # SB3 seeds NumPy and PyTorch, which take a 32-bit seed.
        model = SAC("MlpPolicy", env, seed=settings.seed % 2**32, verbose=0, **train.sac.arguments())
        print(f"train: new run {folder}", flush=True)

    remaining = train.train.total_steps - model.num_timesteps
    if remaining <= 0:
        print(f"train: already at step {model.num_timesteps} of {train.train.total_steps}", flush=True)
        env.close()
        return 0
    progress = Progress(folder, train.train.save_every, train.train.total_steps, episode)
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
