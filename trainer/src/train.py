"""The training run: SAC learns the policy on the loop (synthetic-loop-dia.md,
The policy learns by SAC; rl-rewrite-plan.md, 3.2).

The policy has to learn from the loop itself for hours, so this runs
Stable-Baselines3's SAC on the group of training instances (env.py) until
it has taken [train] total_steps, every instance's steps counting, and
keeps the run recoverable: every [train] save_every steps it saves the
model, its replay buffer and their manifest (run_folder.py), written on a
second thread while training goes on, so a run that fails or is stopped
(Ctrl+C) resumes from its last save.

A session reads [train] alone first (the run folder and the first
episode's seed come from it), launches the run and asks it for the run's
values, and only then loads the rest of the gym file, whose plugins need
them; so a gym file with an error is found after the launch, and the run
is stopped in order. A resume continues the same folder. It reads the
saved manifest and refuses, with the reason, a changed observation,
changed end conditions or changed run values (run_folder.refusal). The reward may change: every stored step is rescored
through the reward plugin the gym file names now, from its stored inputs
(buffer.py), so the stored steps teach the reward the run continues
under. The model is loaded and rescored before the launch, from the run's
values the manifest saved, which the refusal holds equal to the live ones:
the loop cannot wait for it (the synthetic Orin closes its socket after
about 40 s without an answer, and a million steps take 35 s to rescore).
[train] total_steps and save_every are read again, so a run is
extended by raising total_steps. A different number of instances regroups
the buffer. Episodes still running at the last save were never logged;
numbering carries on after the highest logged.

usage (from the repo root):
  uv run --project trainer python trainer/src/train.py [--settings FILE] [--resume [RUN_FOLDER]]
--resume alone continues the most recent run in [train] runs_dir that has a save.
"""

import argparse
import csv
import shutil
import sys
import time
from pathlib import Path

from stable_baselines3 import SAC
from stable_baselines3.common.callbacks import BaseCallback

from buffer import InputsReplayBuffer, regroup, rescore
from contracts import RunValues
from env import LoopVecEnv
from link import RunFailed, stop_unused
from link.real import RealLink
from record import Record
from run_folder import RunFolder, Saver, build_manifest, refusal
from settings import DEFAULT_PATH, SEED_LIMIT, Gym, Settings, read_train

LATEST = Path("<latest>")  # --resume with no folder

COLUMNS = ["episode", "instance", "seed", "steps", "length", "ended", "reward", "reward_mean", "wall_s"]


class Progress(BaseCallback):
    """Logs each finished episode, and saves every save_every steps."""

    def __init__(self, folder: RunFolder, manifest, save_every: int, total_steps: int, instances: int,
                 first_step: int) -> None:
        super().__init__()
        self.folder = folder
        # The instances step together, one buffer row a step, so training
        # writes at most save_every / instances rows (rounded up) between two
        # saves, and one more on the step that crosses the multiple.
        self.saver = Saver(folder, manifest, -(-save_every // instances) + 1)
        self.save_every = save_every
        self.total_steps = total_steps
        self.first_step = first_step
        self.session_started = time.perf_counter()
        # The instances step together, so the count moves by their number:
        # a save is due each time it crosses a multiple of save_every.
        self.saved_block = first_step // save_every
        self.saved_at = None
        self.file = open(folder.episodes, "a", newline="", encoding="utf-8")
        self.rows = csv.writer(self.file)
        if self.file.tell() == 0:
            self.rows.writerow(COLUMNS)
        self.length = [0] * instances
        self.reward = [0.0] * instances
        self.started = [time.perf_counter()] * instances

    def save(self) -> None:
        self.saver.start(self.model, self.num_timesteps)
        self.saved_at = self.num_timesteps

    def _on_step(self) -> bool:
        for k, info in enumerate(self.locals["infos"]):
            self.length[k] += 1
            self.reward[k] += float(self.locals["rewards"][k])
            if self.locals["dones"][k]:
                self.episode_ended(k, info)
        return True

    def _on_rollout_end(self) -> None:
        # Here, not in _on_step: SB3 calls on_step before it stores the
        # step in the replay buffer, so a save there would lose that step.
        block = self.num_timesteps // self.save_every
        if block > self.saved_block:
            self.save()
            self.saved_block = block

    def episode_ended(self, k: int, info: dict) -> None:
        ended = info["end"] or "time limit"
        wall = time.perf_counter() - self.started[k]
        mean = self.reward[k] / self.length[k]
        self.rows.writerow([info["episode_number"], k, info["seed"], self.num_timesteps, self.length[k], ended,
                            f"{self.reward[k]:.3f}", f"{mean:.4f}", f"{wall:.1f}"])
        self.file.flush()
        elapsed = time.perf_counter() - self.session_started
        done = max(self.num_timesteps - self.first_step, 1)
        left = elapsed / done * max(self.total_steps - self.num_timesteps, 0)
        print(f"train: episode {info['episode_number']:4d}  instance {k}  "
              f"step {self.num_timesteps:7,d}/{self.total_steps:,d}  {self.length[k]:4d} steps  {ended:<16}  "
              f"reward {self.reward[k]:8.2f}  mean {mean:.4f}  elapsed {clock(elapsed)}  left {clock(left)}",
              flush=True)
        self.length[k], self.reward[k], self.started[k] = 0, 0.0, time.perf_counter()

    def _on_training_end(self) -> None:
        self.file.close()


def clock(seconds: float) -> str:
    minutes = round(seconds / 60)
    return f"{minutes // 60}h{minutes % 60:02d}m"


def next_episode(folder: RunFolder) -> int:
    """The number after the highest episode episodes.csv logged, 0 if none."""
    if not folder.episodes.is_file():
        return 0
    with open(folder.episodes, newline="", encoding="utf-8") as f:
        numbers = [int(row["episode"]) for row in csv.DictReader(f)]
    return max(numbers) + 1 if numbers else 0


def load_saved(folder: RunFolder, gym: Gym, instances: int) -> SAC:
    """The saved model and its replay buffer, every stored step rescored
    through `gym`'s reward and the buffer regrouped for `instances`; the
    environment is set after the launch (SAC.set_env)."""
    model = SAC.load(folder.model, device=gym.sac.device)
    model.load_replay_buffer(folder.buffer)
    if not isinstance(model.replay_buffer, InputsReplayBuffer):
        raise SystemExit(f"train: {folder.buffer} holds no step inputs, so it cannot be rescored")
    rescored, before, after = rescore(model.replay_buffer, gym.reward.impl)
    print(f"train: rescored {rescored:,d} stored steps with [reward] {gym.reward.name}, "
          f"mean {before:.4f} -> {after:.4f} a step", flush=True)
    if model.replay_buffer.n_envs != instances:
        was = model.replay_buffer.n_envs
        model.replay_buffer, kept = regroup(model.replay_buffer, instances)
        model.n_envs = instances  # set_env requires the model's count to match the environment's
        print(f"train: the replay buffer, saved with {was} instance(s), is regrouped for {instances}, "
              f"keeping the newest {kept:,d} steps", flush=True)
    return model


def real_link(settings: Settings, instances: int, folder: RunFolder, steps: int):
    def open_link(seed: int) -> RealLink:
        return RealLink(settings, seed, instances, log=folder.log, out=folder.loop_out(steps))
    return open_link


def train(settings: Settings, resume: Path | None, link=real_link) -> int:
    """One session of a training run: a new run, or `resume` (a run folder,
    or LATEST). `link(settings, instances, folder, steps)` gives the
    function that opens the session's link from its first seed."""
    instances = settings.instances
    runs_dir = read_train(settings.gym).runs_dir
    if resume is not None:
        folder = RunFolder.latest(runs_dir) if resume == LATEST else RunFolder(resume.resolve())
        saved = folder.read_manifest()
        steps = saved["steps"]
        model = load_saved(folder, Gym.load(settings.gym, settings.facts(RunValues.from_message(
            saved["run_values"]))), instances)
    else:
        folder = RunFolder.create(runs_dir)
        saved, steps, model = None, 0, None
    episode = next_episode(folder)

    # Episode e runs with seed (seed + e), as the environment numbers them.
    opened = link(settings, instances, folder, steps)((settings.seed + episode) % SEED_LIMIT)
    try:
        values = opened.ask()
        gym = Gym.load(settings.gym, settings.facts(values))
        reason = refusal(saved, build_manifest(settings, gym, values, steps, instances)) if saved else None
        if reason:
            raise SystemExit(f"train: {folder.path}: {reason}")
    except BaseException:
        stop_unused(opened)
        raise

    def manifest(steps: int) -> dict:
        return build_manifest(settings, gym, values, steps, instances)

    shutil.copyfile(settings.path, folder.path / f"settings_from_{steps}.toml")
    shutil.copyfile(gym.path, folder.path / f"gym_from_{steps}.toml")
    record = Record(folder.captures(steps), gym.reward.impl.factors)
    env = LoopVecEnv(gym, opened, instances, settings.seed, first_episode=episode, record=record)

    if model is not None:
        model.set_env(env)
        print(f"train: resuming {folder.path} at step {model.num_timesteps}, episode {episode}, "
              f"{instances} instance(s)", flush=True)
    else:
        # SB3 seeds NumPy and PyTorch, which take a 32-bit seed.
        model = SAC("MlpPolicy", env, seed=settings.seed % 2**32, verbose=0,
                    replay_buffer_class=InputsReplayBuffer, **gym.sac.arguments())
        print(f"train: new run {folder.path}, {instances} instance(s)", flush=True)

    remaining = gym.train.total_steps - model.num_timesteps
    if remaining <= 0:
        print(f"train: already at step {model.num_timesteps} of {gym.train.total_steps}", flush=True)
        record.close()
        return 0
    progress = Progress(folder, manifest, gym.train.save_every, gym.train.total_steps, instances,
                        model.num_timesteps)
    code = 0
    try:
        model.learn(total_timesteps=remaining, callback=progress, reset_num_timesteps=resume is None)
    except KeyboardInterrupt:
        print("train: stopped (Ctrl+C)", flush=True)
        code = 130
    except RunFailed as error:
        print(f"train: a run failed, training ends: {error}", flush=True)
        code = 1
    finally:
        progress.saver.wait()
        if progress.saved_at != model.num_timesteps:
            progress.save()
            progress.saver.wait()
        try:
            env.close()
        except Exception as error:  # the run may already be gone after Ctrl+C or a failure
            print(f"train: closing the environment: {error}", file=sys.stderr, flush=True)
        record.close()
    if code != 0:
        print(f"train: resume with --resume {folder.path}", flush=True)
    return code


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--settings", type=Path, default=DEFAULT_PATH,
                        help="the trainer's settings.toml (default: trainer/settings.toml)")
    parser.add_argument("--resume", type=Path, nargs="?", const=LATEST,
                        help="a run folder to continue from its last save; alone, the most recent run")
    args = parser.parse_args()
    settings = Settings.load(args.settings)
    settings.require_built()
    return train(settings, args.resume)


if __name__ == "__main__":
    sys.exit(main())
