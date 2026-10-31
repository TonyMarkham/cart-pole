"""The linear rate: the command is a x the top pulse rate (the run's
values), rounded to the i32 (synthetic-loop-dia.md, The trainer's
environment). The action is one value a; beyond [-1, 1] it is clipped, so
the command never asks past the top pulse rate, and the clipped a is what
the history holds.
"""

from dataclasses import dataclass

import numpy as np

import checks
from contracts import Command
from plugins import History, RunFacts


@dataclass(frozen=True)
class LinearRate:
    top_pulse_rate: int  # pulses/s, the run's: a = 1 asks for it
    history = 0  # reads no past commands

    @staticmethod
    def from_table(table: dict, facts: RunFacts, where: str) -> "LinearRate":
        checks.only(table, set(), where.rstrip("."))
        return LinearRate(facts.top_pulse_rate)

    def command(self, action, history: History) -> tuple[float, Command]:
        a = float(np.clip(np.asarray(action, dtype=np.float64).reshape(-1)[0], -1.0, 1.0))
        return a, Command(round(a * self.top_pulse_rate))


PLUGIN = LinearRate
