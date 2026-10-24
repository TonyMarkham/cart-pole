"""The group of training instances as one vectorized environment.

The policy learns faster from several copies of the loop at once, so
train.py runs settings.toml's [orchestrator] instances training instances
(synthetic-loop-dia.md, Training instances) in one run, behind
Stable-Baselines3's VecEnv: each step gives every instance its command and
returns every instance's next pool, reward and end.

- reset() starts the run (1) and returns every instance's first pool; on an
  open run it resets every instance in place.
- step(actions) answers every instance with continue and its command (18)
  and reads every next pool (17). An instance whose episode ends there (the
  end crash, or gym.toml's [env] max_episode_steps) is answered at once,
  alone, with reset and its next episode's seed (synthetic-loop-dia.md,
  Reset in place), and its new episode's first pool is what this step
  returns for it, its last pool going in info["terminal_observation"], as
  SB3 expects of a VecEnv. The other instances stay pending meanwhile, so
  the next commands are all chosen from current pools.
- close() answers every instance with stop and waits for the run to close.

Episodes are numbered across the run, in the order they start: instance k's
first is first_episode + k, then each new one the next number. Episode e
runs with seed settings.toml's seed + e, so the latency draws differ between
episodes and the run repeats. The observation, the action, the reward and
the end crash are LoopEnv's (loop_env.py).
"""

from pathlib import Path

import numpy as np
from gymnasium import spaces
from stable_baselines3.common.vec_env import VecEnv

from gym_settings import GymSettings
from launch import Run, RunFailed
from loop_env import command, observation, observation_space
from output import Answer
from pool import Pool
from reward import score
from settings import SEED_LIMIT, Settings


class LoopVecEnv(VecEnv):
    def __init__(self, settings: Settings, gym_settings: GymSettings, log: Path | None = None,
                 first_episode: int = 0) -> None:
        self.settings = settings
        self.gym_settings = gym_settings
        self.log = log  # the run's log is appended here, else it goes to the console (launch.py)
        self.render_mode = None  # VecEnv's constructor reads it through get_attr
        super().__init__(
            settings.instances,
            observation_space(settings.rnd_delay_max),
            spaces.Box(-1.0, 1.0, shape=(1,), dtype=np.float32),
        )
        self.run: Run | None = None
        self.next_episode = first_episode  # the next episode's number
        self.episode = [0] * self.num_envs  # each instance's current episode
        self.length = [0] * self.num_envs  # steps in it so far
        self.actions: np.ndarray | None = None

    def seed_of(self, episode: int) -> int:
        return (self.settings.seed + episode) % SEED_LIMIT

    def _start_episode(self, k: int) -> None:
        self.episode[k] = self.next_episode
        self.length[k] = 0
        self.next_episode += 1

    def reset(self) -> np.ndarray:
        if self.run is None:
            # Instance k's first episode runs with --seed + k, which is
            # seed_of(first + k).
            first = self.next_episode
            self.run = Run(self.settings, self.seed_of(first), self.log)
            for k in range(self.num_envs):
                self._start_episode(k)
        else:
            answers = {}
            for k in range(self.num_envs):
                self._start_episode(k)
                answers[k] = Answer.reset(self.seed_of(self.episode[k]))
            self.run.answer(answers)
        pools = self._next_pools()
        self.reset_infos = [{"episode_number": self.episode[k], "seed": self.seed_of(self.episode[k])}
                            for k in range(self.num_envs)]
        return np.stack([observation(pools[k]) for k in range(self.num_envs)])

    def step_async(self, actions: np.ndarray) -> None:
        self.actions = actions

    def step_wait(self):
        if self.run is None or self.actions is None:
            raise RuntimeError("step() before reset(), or after the run failed")
        rates = [command(self.actions[k]) for k in range(self.num_envs)]
        self.run.answer({k: Answer.go(rates[k]) for k in range(self.num_envs)})
        pools = self._next_pools()
        observations = np.stack([observation(pools[k]) for k in range(self.num_envs)])
        rewards = np.zeros(self.num_envs, dtype=np.float32)
        dones = np.zeros(self.num_envs, dtype=bool)
        infos: list[dict] = []
        ended = []
        for k in range(self.num_envs):
            scored = score(pools[k], self.gym_settings.reward, self.gym_settings.end_crash)
            self.length[k] += 1
            truncated = not scored.crashed and self.length[k] >= self.gym_settings.max_episode_steps
            rewards[k] = scored.reward
            dones[k] = scored.crashed or truncated
            infos.append({
                "instance": k,
                "episode_number": self.episode[k],
                "seed": self.seed_of(self.episode[k]),
                "id": self.run.next_ids[k] - 1,
                "command": rates[k],
                "pole": scored.pole,
                "cart": scored.cart,
                "crashed": scored.crashed,
                # SB3 bootstraps a time-limit end and not a crash
                # (synthetic-loop-dia.md, The policy learns by SAC).
                "TimeLimit.truncated": truncated,
            })
            if dones[k]:
                infos[k]["terminal_observation"] = observations[k]
                ended.append(k)
        if ended:
            answers = {}
            for k in ended:
                self._start_episode(k)
                answers[k] = Answer.reset(self.seed_of(self.episode[k]))
            self.run.answer(answers)
            first = self._next_pools()
            for k in ended:
                observations[k] = observation(first[k])
        return observations, rewards, dones, infos

    def close(self) -> None:
        if self.run is None:
            return
        run, self.run = self.run, None
        try:
            if not run.stopped:
                run.answer({k: Answer.stop() for k in range(self.num_envs)})
            run.finish()
        finally:
            run.kill()

    def _next_pools(self) -> dict[int, Pool]:
        assert self.run is not None
        try:
            return self.run.next_pools()
        except RunFailed:
            # The run is over; nothing remains to stop.
            self.run.kill()
            self.run = None
            raise

    # The VecEnv methods for reaching into separate environments: the
    # instances are one run here, so they share this object's attributes.
    def _indices(self, indices) -> list[int]:
        if indices is None:
            return list(range(self.num_envs))
        if isinstance(indices, int):
            return [indices]
        return list(indices)

    def get_attr(self, attr_name, indices=None):
        return [getattr(self, attr_name) for _ in self._indices(indices)]

    def set_attr(self, attr_name, value, indices=None):
        setattr(self, attr_name, value)

    def env_method(self, method_name, *method_args, indices=None, **method_kwargs):
        raise NotImplementedError(f"the instances have no separate environments to call {method_name} on")

    def env_is_wrapped(self, wrapper_class, indices=None):
        return [False for _ in self._indices(indices)]
