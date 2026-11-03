"""The trained policy, run with numpy alone: no PyTorch on the Orin
(real-loop-dia.md, The policy on the Orin).

The policy is the actor of a SAC training run in the synthetic loop, written by
the trainer's export_policy.py as policy.npz. On the rig it gives its best guess
every time, with no randomness:

    h1   = relu(W1 x + b1)      x: the 36 inputs; W1 256 x 36
    h2   = relu(W2 h1 + b2)     W2 256 x 256
    mean = W3 h2 + b3           W3 1 x 256
    a    = tanh(mean)           in [-1, 1]

relu(v) is max(v, 0), each number on its own. The command is
round(a x top_pulse_rate) pulses/s.

The 36 inputs, as the policy trained on them: the pool's 6 entries, newest
first, each its age in ms divided by age_unit_ms (0, then negative), then
Pivot's (x, z) and End's (x, z) in metres, (-1, -1) for a marker not found; then
the actions a of the last past_commands commands, newest first, 0 before the
first. The values the inputs and the command are built with come from the file,
so they are always the trained run's.

The file also carries inputs from the run with the trainer's own answer to each;
`check` works them out here and gives the worst difference.
"""

from dataclasses import dataclass
from pathlib import Path

import numpy as np

ENTRIES = 6  # the pool's entries (synthetic-loop-dia.md, Contracts, the pool)
ROW = 5  # an entry: its age, then Pivot's and End's (x, z)
LAYERS = ("hidden_1", "hidden_2", "output")


class PolicyError(ValueError):
    """The policy file cannot be used. The message says why."""


@dataclass(frozen=True)
class Policy:
    weights: tuple[tuple[np.ndarray, np.ndarray], ...]  # (W, b) per layer, in LAYERS order
    top_pulse_rate: int  # pulses/s: a = 1 asks for it
    age_unit_ms: float  # each age is divided by this before the network sees it
    past_commands: int
    command_rate_min_hz: float  # the band of command rates the policy trained with
    command_rate_max_hz: float
    check_inputs: np.ndarray
    check_actions: np.ndarray
    source: str  # the run, the model and the commit it came from
    path: Path

    @property
    def inputs(self) -> int:
        return ENTRIES * ROW + self.past_commands

    @staticmethod
    def load(path: Path) -> "Policy":
        if not path.is_file():
            raise PolicyError(f"{path}: no policy file: write it with the trainer's export_policy.py and copy it here")
        with np.load(path, allow_pickle=False) as f:
            weights = tuple((f[f"{layer}_weight"].astype(np.float64), f[f"{layer}_bias"].astype(np.float64))
                            for layer in LAYERS)
            policy = Policy(weights, int(f["top_pulse_rate"]), float(f["age_unit_ms"]), int(f["past_commands"]),
                            float(f["command_rate_min_hz"]), float(f["command_rate_max_hz"]),
                            f["check_inputs"], f["check_actions"], str(f["source"]), path)
        if policy.weights[0][0].shape[1] != policy.inputs or policy.weights[-1][0].shape[0] != 1:
            raise PolicyError(f"{path}: the network reads {policy.weights[0][0].shape[1]} inputs and gives "
                              f"{policy.weights[-1][0].shape[0]}, not {policy.inputs} and 1")
        return policy

    def action(self, inputs: np.ndarray) -> np.ndarray:
        """a for each row of inputs, or for one row of 36."""
        x = np.asarray(inputs, dtype=np.float64)
        (w1, b1), (w2, b2), (w3, b3) = self.weights
        h1 = np.maximum(x @ w1.T + b1, 0.0)
        h2 = np.maximum(h1 @ w2.T + b2, 0.0)
        return np.tanh(h2 @ w3.T + b3)[..., 0]

    def command(self, a: float) -> int:
        """The pulse rate for a, in pulses/s."""
        return round(float(np.clip(a, -1.0, 1.0)) * self.top_pulse_rate)

    def check(self) -> float:
        """The worst difference between this file's answers worked out here and the
        trainer's for the same inputs. The trainer's export allowed 1e-5."""
        return float(np.abs(self.action(self.check_inputs) - self.check_actions).max())
