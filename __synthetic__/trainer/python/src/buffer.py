"""The replay buffer, which keeps every stored step's inputs beside it, so
any reward can be recomputed onto the experience already gathered
(rl-rewrite-plan.md, R5 and R6).

`InputsReplayBuffer` is SB3's ReplayBuffer with one more array, `inputs`:
each step's `StepInputs`, flat, at the same row and column as its
observation, action and reward. SB3 2.9.0 hands `add` the step's infos
(buffers.py:247), and the environment puts the inputs there (env.py). It
is pickled and loaded with the buffer, by SB3's own save and load.

`rescore` recomputes every stored reward through a reward plugin from the
stored inputs: the same `score` the environment calls live, so live,
resumed and offline scoring are one implementation (R6).

`frozen` is the buffer as it stands at a save, for a save written while
training goes on (run_folder.py, Saver).
"""

import copy

import numpy as np
from stable_baselines3.common.buffers import ReplayBuffer

from contracts import StepInputs
from plugins import Reward

# The arrays holding one entry per stored step, row by column.
ROWS = ("observations", "next_observations", "actions", "rewards", "dones", "timeouts", "inputs")


class InputsReplayBuffer(ReplayBuffer):
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)
        self.inputs = np.zeros((self.buffer_size, self.n_envs, StepInputs.SIZE), dtype=np.float64)

    def add(self, obs, next_obs, action, reward, done, infos) -> None:
        # Before super().add, which moves pos on to the next row.
        self.inputs[self.pos] = np.stack([info["inputs"].to_array() for info in infos])
        super().add(obs, next_obs, action, reward, done, infos)

    def stored(self) -> int:
        """Rows holding steps."""
        return self.buffer_size if self.full else self.pos


def rescore(buffer: InputsReplayBuffer, reward: Reward) -> tuple[int, float, float]:
    """Every stored step's reward, scored again from its stored inputs.
    Returns the steps rescored and the mean reward a step before and after."""
    rows = buffer.stored()
    if rows == 0:
        return 0, 0.0, 0.0
    before = float(buffer.rewards[:rows].mean())
    for i in range(rows):
        for k in range(buffer.n_envs):
            buffer.rewards[i, k] = reward.score(StepInputs.from_array(buffer.inputs[i, k]))[0]
    return rows * buffer.n_envs, before, float(buffer.rewards[:rows].mean())


def regroup(old: InputsReplayBuffer, instances: int) -> tuple[InputsReplayBuffer, int]:
    """The buffer laid out for `instances` columns. SAC samples a row and a
    column at random, and every stored step carries its own observation,
    action, reward, next observation, end and inputs, so the steps can be
    poured into new columns in time order. The capacity in steps stays the
    same; the oldest steps that would leave a part-filled row are dropped,
    since every column of a row is sampled. Returns the new buffer and the
    steps kept."""
    rows = old.stored()
    order = np.roll(np.arange(rows), -old.pos) if old.full else np.arange(rows)  # time order
    new = InputsReplayBuffer(old.buffer_size * old.n_envs, old.observation_space, old.action_space,
                             device=old.device, n_envs=instances, optimize_memory_usage=False,
                             handle_timeout_termination=old.handle_timeout_termination)
    steps = min(rows * old.n_envs, new.buffer_size * instances)
    kept_rows = steps // instances
    kept = kept_rows * instances
    for name in ROWS:
        array = getattr(old, name)
        flat = array[:rows][order].reshape(rows * old.n_envs, *array.shape[2:])
        getattr(new, name)[:kept_rows] = flat[len(flat) - kept:].reshape(kept_rows, instances, *flat.shape[1:])
    new.pos = kept_rows % new.buffer_size
    new.full = kept_rows == new.buffer_size
    return new, kept


class Spliced:
    """An array kept as consecutive row blocks, some live, some copied. It
    pickles as the one array they make: unpickling joins the blocks with
    np.concatenate, so a resume loads a plain array."""

    def __init__(self, blocks: tuple[np.ndarray, ...]) -> None:
        self.blocks = blocks

    def __reduce__(self):
        return np.concatenate, (self.blocks,)


def frozen(buffer: InputsReplayBuffer, rows_ahead: int) -> InputsReplayBuffer:
    """The buffer as it stands now, for a save written while training goes
    on. Training writes one row a step, at pos, then moves pos on, wrapping
    at the end; no other step changes a row (SAC only samples). So while
    the save is written, training reaches at most the `rows_ahead` rows from
    pos: those are copied now, every other row is pickled from the live
    buffer, and no row can change under the save. The copy is shallow, so
    pos and full are frozen with it. A stored step is about 1,080 bytes
    (a 1,000,000-step buffer pickles to 1,079,997,856 bytes, run
    20261029_022705), so the rows of 10,000 steps copy about 11 MB."""
    snapshot = copy.copy(buffer)
    n, start = buffer.buffer_size, buffer.pos
    end = start + rows_ahead
    for name in ROWS:
        live = getattr(buffer, name)
        if rows_ahead >= n:
            blocks = (live.copy(),)
        elif end <= n:
            blocks = (live[:start], live[start:end].copy(), live[end:])
        else:
            blocks = (live[:end - n].copy(), live[end - n:start], live[start:].copy())
        setattr(snapshot, name, Spliced(blocks))
    return snapshot
