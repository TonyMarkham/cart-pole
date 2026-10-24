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
- loop_from_<steps>/instance_k/, the orchestrator's output for each launch
  (its --out, in place of settings.toml's): every frame, the true states
  (states.csv, MuJoCo's cart and pole per step) and observations.jsonl.
  <steps> is where that session began, 0 for a new run, so a resume writes
  beside the sessions before it, never over them (the first run's were
  replaced by the next run's, trainer-observations.md). About 21 KB a
  frame, so 200,000 steps is about 4 GB;
- episodes.csv, one row per finished episode, in the order they finish: its
  number, the instance it ran on, its seed, the run's steps so far, its
  length, how it ended (time limit or end crash), its total reward, and the
  pole's height ((1 - cos theta) / 2: 0 hanging, 1 upright, reward.py) at
  its highest and on average.

Episode e runs with seed settings.toml's seed + e (vec_env.py), so the
latency draws differ between episodes and the run repeats.

A resumed run continues the same folder: the model and buffer from its last
save, with the SAC settings it was saved with; [train] total_steps and
save_every are read again, so a run can be extended by raising total_steps.
Its instances may differ from when it was saved: the replay buffer holds one
column per instance, and is regrouped into the new number (regroup). Episodes still running at the last save were
never logged, and numbering carries on after the highest logged.

A resume also rescores the replay buffer with gym.toml's [reward] as it is
now, so a run can carry on under a changed reward without its stored steps
teaching the old one (trainer-observations.md: the slow-pole reward). This
is exact because a step's reward is a function of the pool it returned
alone (reward.py), and the buffer keeps that pool as the step's next
observation (an episode's last pool too, SB3 storing the terminal
observation there). The end crash is read from the same pool, so the
rescoring checks that every stored crash is still one and every other step
still is not; a changed [end_crash] fails that check, and the resume is
refused. Buffers saved before vec_env.py's terminal-observation fix
(2026-10-24, the first run's) hold the next episode's first pool in place
of an episode's last, so those steps alone cannot be rescored: they are
dropped from the buffer (regroup), and every step that remains is exact.
The settings it resumes with are copied into the run's folder as
settings_from_<steps>.toml and gym_from_<steps>.toml.

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

import numpy as np
from stable_baselines3 import SAC
from stable_baselines3.common.buffers import ReplayBuffer
from stable_baselines3.common.callbacks import BaseCallback

from gym_settings import GymSettings
from launch import RunFailed
from pool import Pool
from reward import score
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


def rescore(model: SAC, gym_settings: GymSettings) -> tuple[int, float, float, np.ndarray]:
    """Each stored step's reward, scored again from its next observation
    with the current [reward]. Returns the steps rescored, the buffer's
    mean reward a step before and after (over the rescored steps), and
    which stored steps lost their last pool (lost_last_pool), not rescored:
    the caller drops them (regroup)."""
    buffer = model.replay_buffer
    rows = buffer.buffer_size if buffer.full else buffer.pos
    # SB3 marks a time-limit end done and timeout; only a crash is done alone.
    done = buffer.dones[:rows].astype(bool)
    crashed = done & ~buffer.timeouts[:rows].astype(bool)
    lost = np.zeros_like(done)
    for i in range(rows):
        for k in range(buffer.n_envs):
            lost[i, k] = done[i, k] and lost_last_pool(buffer, i, k)
    before = float(buffer.rewards[:rows][~lost].mean())
    for i in range(rows):
        for k in range(buffer.n_envs):
            if lost[i, k]:
                continue
            pool = Pool.from_flat(buffer.next_observations[i, k].reshape(-1).tolist())
            scored = score(pool, gym_settings.reward, gym_settings.end_crash)
            if scored.crashed != crashed[i, k]:
                raise SystemExit(
                    f"train: stored step {i} of instance {k} was {'a' if crashed[i, k] else 'no'} crash and "
                    f"gym.toml's [end_crash] now scores it {'a' if scored.crashed else 'no'} crash: a changed "
                    f"end crash changes where episodes end, so start a new run"
                )
            buffer.rewards[i, k] = scored.reward
    rescored = int((~lost).sum())
    return rescored, before, float(buffer.rewards[:rows][~lost].mean()), lost


def regroup(model: SAC, instances: int, keep: np.ndarray) -> int:
    """The replay buffer laid out for `instances` columns, holding only the
    stored steps `keep` marks. SB3's buffer holds one column per instance,
    but SAC samples a row and a column at random, and every stored step
    carries its own pool, action, reward, next pool and end, so the steps
    can be poured into new columns in time order. The capacity in steps
    stays the same; the oldest steps that would leave a part-filled row
    (fewer than `instances`) are dropped too, since every column of a row
    is sampled. Run after rescore, which reads each instance's steps in
    their own column. Returns the steps kept."""
    old = model.replay_buffer
    rows = old.buffer_size if old.full else old.pos
    # Time order: a full buffer's oldest row is at pos.
    order = np.roll(np.arange(rows), -old.pos) if old.full else np.arange(rows)
    chosen = keep[order].reshape(-1)
    new = ReplayBuffer(old.buffer_size * old.n_envs, old.observation_space, old.action_space,
                       device=old.device, n_envs=instances, optimize_memory_usage=False,
                       handle_timeout_termination=old.handle_timeout_termination)
    steps = min(int(chosen.sum()), new.buffer_size * instances)
    kept_rows = steps // instances
    kept = kept_rows * instances
    for name in ("observations", "next_observations", "actions", "rewards", "dones", "timeouts"):
        array = getattr(old, name)
        flat = array[:rows][order].reshape(rows * old.n_envs, *array.shape[2:])[chosen]
        getattr(new, name)[:kept_rows] = flat[len(flat) - kept:].reshape(kept_rows, instances, *flat.shape[1:])
    new.pos = kept_rows % new.buffer_size
    new.full = kept_rows == new.buffer_size
    model.replay_buffer = new
    model.n_envs = instances
    return kept


def lost_last_pool(buffer, i: int, k: int) -> bool:
    """Whether episode-ending step i of instance k stored the next episode's
    first pool as its next observation (the bug vec_env.py fixed): then it
    equals the observation the following step started from. A real last
    pool never does, the next episode starting at rest from the start."""
    j = (i + 1) % buffer.buffer_size
    if not buffer.full and j >= buffer.pos:
        return False
    return bool(np.array_equal(buffer.next_observations[i, k], buffer.observations[j, k]))


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
        steps, before, after, lost = rescore(model, gym_settings)
        saved_instances = model.replay_buffer.n_envs
        if lost.any():
            print(f"train: {int(lost.sum())} stored episode ends lost their last pool (saved before "
                  f"vec_env.py's fix) and are dropped", flush=True)
        if saved_instances != env.num_envs or lost.any():
            kept = regroup(model, env.num_envs, ~lost)
            print(f"train: the replay buffer, saved with {saved_instances} instance(s), is regrouped for "
                  f"{env.num_envs}, keeping the newest {kept:,d} of {steps:,d} rescored steps", flush=True)
        shutil.copyfile(args.settings, folder / f"settings_from_{model.num_timesteps}.toml")
        shutil.copyfile(settings.gym, folder / f"gym_from_{model.num_timesteps}.toml")
        print(f"train: resuming {folder} at step {model.num_timesteps}, episode {episode}, "
              f"{env.num_envs} instance(s); rescored {steps:,d} stored steps with gym.toml's [reward], "
              f"mean {before:.3f} -> {after:.3f} a step", flush=True)
    else:
        # SB3 seeds NumPy and PyTorch, which take a 32-bit seed.
        model = SAC("MlpPolicy", env, seed=settings.seed % 2**32, verbose=0, **train.sac.arguments())
        print(f"train: new run {folder}, {env.num_envs} instance(s)", flush=True)

    remaining = train.train.total_steps - model.num_timesteps
    if remaining <= 0:
        print(f"train: already at step {model.num_timesteps} of {train.train.total_steps}", flush=True)
        return 0
    env.out = folder / f"loop_from_{model.num_timesteps}"
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
