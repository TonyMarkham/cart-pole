"""The synthetic loop as a Gymnasium environment: one run per episode.

A learning algorithm needs the loop behind the interface it already speaks,
a Gymnasium Env, so the trainer's environment wraps the run
(synthetic-loop-dia.md, The trainer's environment; trainer-plan.md T8):

- reset() starts a run (1) and returns pool 0 (17);
- step(a) answers the pending pool with continue and the command (18), and
  returns the next pool (17);
- reset() or close() while a run is open answers the pending pool with
  stop and waits for the run to close (26); a run that fails raises
  RunFailed, which ends training.

The observation is the pool, 6 x 9 floats: per entry, newest first, its age
in ms (0, then negative) and the 4 markers' (x, y), (-1, -1) where not
detected. The action is one value a in [-1, 1]; the command is a x 56,666,
the synthetic Arduino's top rate, rounded to the i32. Beyond [-1, 1] the
action is clipped, so the command never asks past the top rate.

Each step's reward is the graded score of the pool it returns (reward.py,
gym.toml [reward]); the Pivot marker past gym.toml's [end_crash] bounds
scores the penalty and terminates the episode. TimeLimit truncates it
after gym.toml [env] max_episode_steps (trainer-plan.md T8a).
The pool reset() returns is not scored (trainer-plan.md T3).

Seeding: reset(seed=s) runs the orchestrator with --seed s; reset() without
a seed uses settings.toml's seed, so every such episode repeats the same
latency draws.

The physics repeats per seed but the frames jitter (synthetic-loop-dia.md,
Determinism), so the environment is registered nondeterministic.
"""

from pathlib import Path

import gymnasium as gym
import numpy as np
from gymnasium import spaces

from gym_settings import MAX_RATE, GymSettings
from launch import Run, RunFailed
from output import Answer
from pool import ENTRIES, FLOATS, MARKERS, Pool
from reward import score
from settings import Settings

ENV_ID = "cart-pole/SyntheticLoop-v0"


# Steps the grid adds to a latency at most: the arrival step is the first
# 2 ms step later than capture + latency.
GRID_MS = 2.0


def observation_space(rnd_delay_max: float) -> spaces.Box:
    """The pool's bounds. Every gap between two entries is at most
    rnd_delay_max + 2 ms (a live gap rounds the latency up to the 2 ms grid;
    a pre-warm gap is a draw <= rnd_delay_max), so the oldest of the 6
    entries is at most 5 gaps old. Markers are in [0, 1], or -1."""
    width = FLOATS // ENTRIES
    low = np.full((ENTRIES, width), -1.0, dtype=np.float32)
    high = np.ones((ENTRIES, width), dtype=np.float32)
    low[:, 0] = -(ENTRIES - 1) * (rnd_delay_max + GRID_MS)
    high[:, 0] = 0.0
    assert width == 1 + 2 * MARKERS
    return spaces.Box(low, high, dtype=np.float32)


def observation(pool: Pool) -> np.ndarray:
    return np.array(pool.flat(), dtype=np.float32).reshape(ENTRIES, FLOATS // ENTRIES)


def command(action) -> int:
    """a in [-1, 1] to the i32 command, pulses/s (18)."""
    a = float(np.clip(np.asarray(action, dtype=np.float64).reshape(-1)[0], -1.0, 1.0))
    return round(a * MAX_RATE)


class LoopEnv(gym.Env):
    metadata = {"render_modes": []}

    def __init__(
        self, settings: Settings | None = None, gym_settings: GymSettings | None = None, log: Path | None = None
    ) -> None:
        self.settings = settings if settings is not None else Settings.load()
        self.gym_settings = gym_settings if gym_settings is not None else GymSettings.load(self.settings.gym)
        self.log = log  # each run's log is appended here, else it goes to the console (launch.py)
        self.observation_space = observation_space(self.settings.rnd_delay_max)
        self.action_space = spaces.Box(-1.0, 1.0, shape=(1,), dtype=np.float32)
        self.run: Run | None = None

    def reset(self, *, seed: int | None = None, options: dict | None = None):
        super().reset(seed=seed)
        self._close_run()
        self.run = Run(self.settings, seed, self.log)
        return observation(self._next_pool()), {"id": 0}

    def step(self, action):
        if self.run is None:
            raise gym.error.ResetNeeded("step() before reset(), or after the run failed")
        rate = command(action)
        self.run.answer(Answer(rate, keep_going=True))
        pool = self._next_pool()
        scored = score(pool, self.gym_settings.reward, self.gym_settings.end_crash)
        info = {
            "id": self.run.observations - 1,
            "command": rate,
            "pole": scored.pole,
            "cart": scored.cart,
            "crashed": scored.crashed,
        }
        return observation(pool), scored.reward, scored.crashed, False, info

    def close(self) -> None:
        self._close_run()

    def _next_pool(self) -> Pool:
        assert self.run is not None
        try:
            return self.run.next_pool()
        except RunFailed:
            # The run is over; nothing remains to stop.
            self.run.kill()
            self.run = None
            raise

    def _close_run(self) -> None:
        """An open run is answered with stop and waited for (22-26)."""
        if self.run is None:
            return
        run, self.run = self.run, None
        try:
            if not run.stopped:
                run.answer(Answer(0, keep_going=False))
            run.finish()
        finally:
            run.kill()


def make(settings: Settings | None = None, gym_settings: GymSettings | None = None, log: Path | None = None) -> gym.Env:
    """The environment through gymnasium.make, so it carries its spec: the
    TimeLimit (gym.toml's episode length) and nondeterministic."""
    settings = settings if settings is not None else Settings.load()
    gym_settings = gym_settings if gym_settings is not None else GymSettings.load(settings.gym)
    if ENV_ID not in gym.registry:
        gym.register(
            id=ENV_ID,
            entry_point=LoopEnv,
            nondeterministic=True,
            max_episode_steps=gym_settings.max_episode_steps,
        )
    return gym.make(ENV_ID, settings=settings, gym_settings=gym_settings, log=log)
