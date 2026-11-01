"""The pool and the last commands (synthetic-loop-dia.md, The trainer's
environment and The policy sees its last commands).

The observation is the pool's 30 floats, per entry newest first its age in
ms (0, then negative) and Pivot's and End's (x, z) in metres, (-1, -1) where
not detected; then the actions a of the last `past_commands` commands, newest
first, 0 before the episode's first. It is built from the pool and the
history alone, so no site can reach it.
"""

from dataclasses import dataclass

import numpy as np
from gymnasium import spaces

import checks
from contracts import ENTRIES, FLOATS, ROW, Pool
from plugins import History, RunFacts


@dataclass(frozen=True)
class PoolAndCommands:
    past_commands: int
    # Every gap between two entries is at most the run's longest gap: a live
    # gap is the slowest command rate's rounded up to the next MuJoCo step
    # (synthetic-loop-dia.md, the grid), and a pre-warm gap is the same draw
    # unrounded.
    longest_gap_ms: float

    @property
    def history(self) -> int:
        return self.past_commands

    @staticmethod
    def from_table(table: dict, facts: RunFacts, where: str) -> "PoolAndCommands":
        checks.only(table, {"past_commands"}, where.rstrip("."))
        return PoolAndCommands(checks.whole(table, "past_commands", where, 0), facts.longest_gap_ms())

    def space(self) -> spaces.Box:
        """Ages from -(5 gaps) to 0, markers in metres within 1, commands in [-1, 1]."""
        low = np.full((ENTRIES, ROW), -1.0, dtype=np.float32)
        high = np.ones((ENTRIES, ROW), dtype=np.float32)
        low[:, 0] = -(ENTRIES - 1) * self.longest_gap_ms
        high[:, 0] = 0.0
        low = np.concatenate([low.reshape(-1), np.full(self.past_commands, -1.0, dtype=np.float32)])
        high = np.concatenate([high.reshape(-1), np.ones(self.past_commands, dtype=np.float32)])
        return spaces.Box(low, high, dtype=np.float32)

    def build(self, pool: Pool, history: History) -> np.ndarray:
        values = np.empty(FLOATS + self.past_commands, dtype=np.float32)
        values[:FLOATS] = pool.flat()
        values[FLOATS:] = history.actions[: self.past_commands]
        return values


PLUGIN = PoolAndCommands
