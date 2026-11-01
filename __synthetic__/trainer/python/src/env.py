"""The group of training instances as one vectorized environment
(Stable-Baselines3's VecEnv), built from the four plugins and a link
(rl-rewrite-plan.md, 3.1 and 3.2). One instance is a group of one.

- It is built on a link already opened (the run's launch, 1) and asked for
  the run's values, which the plugins were built with (gym.facts).
- The first reset() returns every instance's first observation; every
  later one resets every instance in place (synthetic-loop-dia.md, Reset
  in place).
- step(actions) maps each action to its command (the action plugin),
  checks it against the top pulse rate (the run's values), answers every
  instance with continue (18) and reads every next capture
  (17). The episode tracker turns each capture into the step's inputs; the
  reward plugin scores them; the end plugins may end the episode; the time
  limit truncates it after [env] max_episode_steps. An instance whose
  episode ends is answered at once, alone, with reset and its next
  episode's seed, and its new episode's first observation is what the step
  returns for it, its last going in info["terminal_observation"], as SB3
  expects of a VecEnv. A time limit is marked "TimeLimit.truncated", so
  SB3 keeps valuing the state it stopped in; an end is terminal.
- close() answers every instance with stop and waits for the run to close.

The observation plugin is given the pool and the command history only,
never the capture, so no site can reach the policy (R1). info carries the
step's inputs, the reward's factors and the end reason, for the replay
buffer (buffer.py), the progress log and the capture record (record.py).

Episodes are numbered across the run in the order they start: instance k's
first is first_episode + k, then each new one the next number; episode e
runs with seed (seed + e) mod 2**64.
"""

import numpy as np
from gymnasium import spaces
from stable_baselines3.common.vec_env import VecEnv

from contracts import Capture
from episode import Episode
from link import Link, RunFailed
from link.answer import Answer
from record import Record
from settings import SEED_LIMIT, Gym


class LoopVecEnv(VecEnv):
    def __init__(self, gym: Gym, link: Link, instances: int, seed: int, first_episode: int = 0,
                 record: Record | None = None) -> None:
        """`link` is open and asked, opened with instance 0's first episode's
        seed, seed_of(first_episode)."""
        self.gym = gym
        self.observation = gym.observation.impl
        self.action = gym.action.impl
        self.reward = gym.reward.impl
        self.endings = [part.impl for part in gym.endings]
        self.top_pulse_rate = gym.facts.top_pulse_rate  # every command's bound, pulses/s
        self.run_seed = seed  # not `seed`: VecEnv.seed() is a method SB3 calls
        self.record = record
        self.render_mode = None  # VecEnv's constructor reads it through get_attr
        super().__init__(instances, self.observation.space(), spaces.Box(-1.0, 1.0, shape=(1,), dtype=np.float32))
        self.link: Link | None = link
        self.started = False  # the first reset() reads the first captures; later ones reset in place
        history = max(self.observation.history, self.action.history)
        self.episodes = [Episode(history) for _ in range(instances)]
        self.next_episode = first_episode
        self.episode = [0] * instances  # each instance's current episode number
        self.actions: np.ndarray | None = None

    def seed_of(self, episode: int) -> int:
        return (self.run_seed + episode) % SEED_LIMIT

    def _number(self, k: int) -> None:
        self.episode[k] = self.next_episode
        self.next_episode += 1

    def _begin(self, k: int, capture: Capture) -> np.ndarray:
        """Capture 0 of instance k's episode: kept, recorded, not scored."""
        self.episodes[k].start(capture)
        if self.record is not None:
            self.record.capture(k, self.episode[k], capture)
        return self.observation.build(capture.pool, self.episodes[k].history)

    def reset(self) -> np.ndarray:
        if self.link is None:
            raise RuntimeError("reset() after the run closed or failed")
        for k in range(self.num_envs):
            self._number(k)
        if self.started:
            self.link.answer({k: Answer.reset(self.seed_of(self.episode[k])) for k in range(self.num_envs)})
        # Else the run's first captures: instance k's first episode runs with
        # --seed + k, seed_of(first_episode + k), as the link was opened.
        self.started = True
        captures = self._next()
        self.reset_infos = [{"episode_number": self.episode[k], "seed": self.seed_of(self.episode[k])}
                            for k in range(self.num_envs)]
        return np.stack([self._begin(k, captures[k]) for k in range(self.num_envs)])

    def step_async(self, actions: np.ndarray) -> None:
        self.actions = actions

    def step_wait(self):
        if self.link is None or self.actions is None:
            raise RuntimeError("step() before reset(), or after the run failed")
        chosen = [self.action.command(self.actions[k], self.episodes[k].history) for k in range(self.num_envs)]
        for _, command in chosen:
            if abs(command.rate) > self.top_pulse_rate:
                raise ValueError(f"command: at most +-{self.top_pulse_rate} pulses/s (the top pulse rate), "
                                 f"got {command.rate}")
        self.link.answer({k: Answer.go(command) for k, (_, command) in enumerate(chosen)})
        captures = self._next()
        observations = np.empty((self.num_envs, *self.observation_space.shape), dtype=np.float32)
        rewards = np.zeros(self.num_envs, dtype=np.float32)
        dones = np.zeros(self.num_envs, dtype=bool)
        infos: list[dict] = []
        ended = []
        for k in range(self.num_envs):
            a, command = chosen[k]
            episode = self.episodes[k]
            inputs = episode.advance(a, command, captures[k])
            reward, factors = self.reward.score(inputs)
            end = next((reason for reason in (e.check(inputs) for e in self.endings) if reason), None)
            truncated = end is None and episode.steps >= self.gym.max_episode_steps
            observations[k] = self.observation.build(captures[k].pool, episode.history)
            rewards[k] = reward
            dones[k] = end is not None or truncated
            infos.append({
                "instance": k, "episode_number": self.episode[k], "seed": self.seed_of(self.episode[k]),
                "id": captures[k].id, "action": a, "command": command.rate, "inputs": inputs,
                "reward": reward, "factors": factors, "end": end, "TimeLimit.truncated": truncated,
            })
            if self.record is not None:
                self.record.capture(k, self.episode[k], captures[k], {
                    "step": inputs.step, "action": a, "command": command.rate, "reward": reward,
                    "factors": factors, "end": end, "truncated": truncated,
                })
            if dones[k]:
                # A copy: observations[k] is overwritten with the next episode's first.
                infos[k]["terminal_observation"] = observations[k].copy()
                ended.append(k)
        if ended:
            for k in ended:
                self._number(k)
            self.link.answer({k: Answer.reset(self.seed_of(self.episode[k])) for k in ended})
            first = self._next()
            for k in ended:
                observations[k] = self._begin(k, first[k])
        return observations, rewards, dones, infos

    def close(self) -> None:
        if self.record is not None:
            self.record.flush()
        if self.link is None:
            return
        link, self.link = self.link, None
        try:
            if not getattr(link, "stopped", False):
                link.answer({k: Answer.stop() for k in range(self.num_envs)})
            link.finish()
        finally:
            link.kill()

    def _next(self) -> dict[int, Capture]:
        assert self.link is not None
        try:
            return self.link.next_captures()
        except RunFailed:
            # The run is over; nothing remains to stop.
            self.link.kill()
            self.link = None
            raise

    # VecEnv's methods for reaching into separate environments: the
    # instances are one run here, so they share this object's attributes.
    def _indices(self, indices) -> list[int]:
        if indices is None:
            return list(range(self.num_envs))
        return [indices] if isinstance(indices, int) else list(indices)

    def get_attr(self, attr_name, indices=None):
        return [getattr(self, attr_name) for _ in self._indices(indices)]

    def set_attr(self, attr_name, value, indices=None):
        setattr(self, attr_name, value)

    def env_method(self, method_name, *method_args, indices=None, **method_kwargs):
        raise NotImplementedError(f"the instances have no separate environments to call {method_name} on")

    def env_is_wrapped(self, wrapper_class, indices=None):
        return [False for _ in self._indices(indices)]
