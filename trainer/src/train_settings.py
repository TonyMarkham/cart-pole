"""The training run's settings, [train] and [sac] in the file settings.toml's
[trainer] gym names (trainer/gym.toml).

They sit beside the environment's own settings so one file holds a whole
variant: the episode, the reward, and how SAC learns on it (trainer-plan.md
T9).
"""

import math
import tomllib
from dataclasses import dataclass
from pathlib import Path

from gym_settings import whole

DEVICES = ("cuda", "cpu")


@dataclass(frozen=True)
class Train:
    """[train]: the run's length, how often it saves, and where."""

    total_steps: int
    save_every: int  # steps between saves of the model and replay buffer
    runs_dir: Path  # absolute; each run gets a folder in it


@dataclass(frozen=True)
class Sac:
    """[sac]: Stable-Baselines3 SAC's arguments, by their SB3 names."""

    device: str
    learning_starts: int
    buffer_size: int
    batch_size: int
    learning_rate: float
    gamma: float
    tau: float
    train_freq: int
    gradient_steps: int
    net_arch: list[int]

    def arguments(self) -> dict:
        return {
            "device": self.device,
            "learning_starts": self.learning_starts,
            "buffer_size": self.buffer_size,
            "batch_size": self.batch_size,
            "learning_rate": self.learning_rate,
            "gamma": self.gamma,
            "tau": self.tau,
            "train_freq": self.train_freq,
            "gradient_steps": self.gradient_steps,
            "policy_kwargs": {"net_arch": list(self.net_arch)},
        }


@dataclass(frozen=True)
class TrainSettings:
    train: Train
    sac: Sac

    @staticmethod
    def load(path: Path) -> "TrainSettings":
        with open(path, "rb") as f:
            raw = tomllib.load(f)
        t = raw["train"]
        train = Train(
            total_steps=whole(path, t, "train.total_steps", 1),
            save_every=whole(path, t, "train.save_every", 1),
            runs_dir=(path.parent / t["runs_dir"]).resolve(),
        )
        s = raw["sac"]
        device = s["device"]
        if device not in DEVICES:
            raise ValueError(f"{path}: sac.device is one of {DEVICES}, got {device!r}")
        net_arch = s["net_arch"]
        if not net_arch or not all(isinstance(n, int) and not isinstance(n, bool) and n >= 1 for n in net_arch):
            raise ValueError(f"{path}: sac.net_arch is a list of layer widths >= 1, got {net_arch!r}")
        sac = Sac(
            device=device,
            learning_starts=whole(path, s, "sac.learning_starts", 0),
            buffer_size=whole(path, s, "sac.buffer_size", 1),
            batch_size=whole(path, s, "sac.batch_size", 1),
            learning_rate=fraction(path, s, "sac.learning_rate"),
            gamma=fraction(path, s, "sac.gamma"),
            tau=fraction(path, s, "sac.tau"),
            train_freq=whole(path, s, "sac.train_freq", 1),
            gradient_steps=gradient_steps(path, s),
            net_arch=net_arch,
        )
        return TrainSettings(train=train, sac=sac)


def gradient_steps(path: Path, table: dict) -> int:
    """Updates per step, >= 1, or -1: as many as the samples the step took
    (one per training instance)."""
    value = table["gradient_steps"]
    if isinstance(value, bool) or not isinstance(value, int) or not (value >= 1 or value == -1):
        raise ValueError(f"{path}: sac.gradient_steps is a whole number >= 1, or -1, got {value!r}")
    return value


def fraction(path: Path, table: dict, name: str) -> float:
    value = table[name.split(".")[-1]]
    if not isinstance(value, (int, float)) or isinstance(value, bool) or not math.isfinite(value) or not 0 < value <= 1:
        raise ValueError(f"{path}: {name} is a number in (0, 1], got {value!r}")
    return float(value)
