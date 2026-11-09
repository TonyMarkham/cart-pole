"""The training run's tables in the gym file: [train] (how long, how often
it saves, where) and [sac] (Stable-Baselines3 SAC's arguments by their SB3
names; synthetic-loop-dia.md, The policy learns by SAC).
"""

import math
from dataclasses import dataclass
from pathlib import Path

import checks

DEVICES = ("cuda", "cpu")


@dataclass(frozen=True)
class Train:
    total_steps: int  # every training instance's steps counting
    save_every: int  # steps between saves of the model, buffer and manifest
    runs_dir: Path  # absolute; each training run gets a folder in it

    @staticmethod
    def from_table(t: dict, folder: Path) -> "Train":
        checks.only(t, {"total_steps", "save_every", "runs_dir"}, "train")
        runs_dir = checks.present(t, "runs_dir", "train.")
        if not isinstance(runs_dir, str):
            raise ValueError(f"train.runs_dir: a folder, relative to the gym file, got {runs_dir!r}")
        return Train(
            total_steps=checks.whole(t, "total_steps", "train.", 1),
            save_every=checks.whole(t, "save_every", "train.", 1),
            runs_dir=(folder / runs_dir).resolve(),
        )


@dataclass(frozen=True)
class Sac:
    device: str
    learning_starts: int
    buffer_size: int
    batch_size: int
    learning_rate: float
    gamma: float
    tau: float
    train_freq: int
    gradient_steps: int  # >= 1, or -1: one per sample the step took
    net_arch: tuple[int, ...]
    ent_coef: str | float  # "auto", "auto_<start>" (tuned from that start), or a fixed weight

    KEYS = {"device", "learning_starts", "buffer_size", "batch_size", "learning_rate", "gamma", "tau",
            "train_freq", "gradient_steps", "net_arch", "ent_coef"}

    @staticmethod
    def read_ent_coef(t: dict) -> str | float:
        """SB3's three forms; SB3 itself only fails on a bad one at the first update."""
        value = checks.present(t, "ent_coef", "sac.")
        if isinstance(value, str):
            start = value.removeprefix("auto_")
            if value == "auto":
                return value
            if start != value:
                try:
                    if math.isfinite(float(start)) and float(start) > 0:
                        return value
                except ValueError:
                    pass
            raise ValueError(f'sac.ent_coef: "auto", "auto_<start above 0>" or a number, got {value!r}')
        return checks.number(t, "ent_coef", "sac.", 0.0, None, low_open=True)

    @staticmethod
    def from_table(t: dict) -> "Sac":
        checks.only(t, Sac.KEYS, "sac")
        device = checks.present(t, "device", "sac.")
        if device not in DEVICES:
            raise ValueError(f"sac.device: one of {DEVICES}, got {device!r}")
        net_arch = checks.present(t, "net_arch", "sac.")
        if (not isinstance(net_arch, list) or not net_arch
                or not all(isinstance(n, int) and not isinstance(n, bool) and n >= 1 for n in net_arch)):
            raise ValueError(f"sac.net_arch: a list of layer widths >= 1, got {net_arch!r}")
        gradient_steps = checks.whole(t, "gradient_steps", "sac.", -1)
        if gradient_steps == 0:
            raise ValueError("sac.gradient_steps: a whole number >= 1, or -1, got 0")
        return Sac(
            device=device,
            learning_starts=checks.whole(t, "learning_starts", "sac.", 0),
            buffer_size=checks.whole(t, "buffer_size", "sac.", 1),
            batch_size=checks.whole(t, "batch_size", "sac.", 1),
            learning_rate=checks.number(t, "learning_rate", "sac.", 0.0, 1.0, low_open=True),
            gamma=checks.number(t, "gamma", "sac.", 0.0, 1.0, low_open=True),
            tau=checks.number(t, "tau", "sac.", 0.0, 1.0, low_open=True),
            train_freq=checks.whole(t, "train_freq", "sac.", 1),
            gradient_steps=gradient_steps,
            net_arch=tuple(net_arch),
            ent_coef=Sac.read_ent_coef(t),
        )

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
            "ent_coef": self.ent_coef,
            "policy_kwargs": {"net_arch": list(self.net_arch)},
        }
