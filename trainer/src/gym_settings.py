"""The trainer's Gymnasium settings, read from the file settings.toml's
[trainer] gym names (trainer/gym.toml).

The environment's own settings (trainer-plan.md T8) grow as the reward, the
end conditions and the learning method are decided, so they live in their
own file, apart from settings.toml's run settings, and variants can sit side
by side.
"""

import math
import tomllib
from dataclasses import dataclass
from pathlib import Path

MAX_RATE = 45_333  # pulses/s: arduino::RIG's top rate (synthetic-loop-dia.md, Numbers)


@dataclass(frozen=True)
class Reward:
    """[reward]: pole_weight x (1 - cos theta) / 2 + cart_weight x (1 - u^2)."""

    pole_weight: float
    cart_weight: float


@dataclass(frozen=True)
class EndCrash:
    """[end_crash]: the Pivot marker's x outside [min, max] is the end crash."""

    pivot_x_min: float
    pivot_x_max: float
    penalty: float  # the crash step's reward


@dataclass(frozen=True)
class Sweep:
    """[sweep]: the naive trainer's scripted action, peak x cos(2 pi n / period)."""

    peak: int  # pulses/s, 0 to MAX_RATE
    period: int  # steps per back-and-forth


@dataclass(frozen=True)
class GymSettings:
    max_episode_steps: int  # [env]: the placeholder episode length, in steps
    reward: Reward
    end_crash: EndCrash
    sweep: Sweep | None  # [sweep], when the file has one

    @staticmethod
    def load(path: Path) -> "GymSettings":
        with open(path, "rb") as f:
            raw = tomllib.load(f)
        steps = whole(path, raw["env"], "env.max_episode_steps", 1)
        reward = Reward(
            pole_weight=number(path, raw["reward"], "reward.pole_weight"),
            cart_weight=number(path, raw["reward"], "reward.cart_weight"),
        )
        end_crash = EndCrash(
            pivot_x_min=number(path, raw["end_crash"], "end_crash.pivot_x_min"),
            pivot_x_max=number(path, raw["end_crash"], "end_crash.pivot_x_max"),
            penalty=number(path, raw["end_crash"], "end_crash.penalty"),
        )
        # Pivot x is normalized to the frame, so the bounds lie in [0, 1].
        if not 0 <= end_crash.pivot_x_min < end_crash.pivot_x_max <= 1:
            raise ValueError(f"{path}: end_crash bounds are 0 <= pivot_x_min < pivot_x_max <= 1, got {end_crash}")
        sweep = None
        if "sweep" in raw:
            peak = whole(path, raw["sweep"], "sweep.peak", 0)
            if peak > MAX_RATE:
                raise ValueError(f"{path}: sweep.peak is at most {MAX_RATE} pulses/s, got {peak}")
            sweep = Sweep(peak=peak, period=whole(path, raw["sweep"], "sweep.period", 1))
        return GymSettings(max_episode_steps=steps, reward=reward, end_crash=end_crash, sweep=sweep)


def whole(path: Path, table: dict, name: str, minimum: int) -> int:
    value = table[name.split(".")[-1]]
    # bool is an int in Python; `true` is a typo, not 1.
    if not isinstance(value, int) or isinstance(value, bool) or value < minimum:
        raise ValueError(f"{path}: {name} is a whole number >= {minimum}, got {value!r}")
    return value


def number(path: Path, table: dict, name: str) -> float:
    value = table[name.split(".")[-1]]
    if not isinstance(value, (int, float)) or isinstance(value, bool) or not math.isfinite(value):
        raise ValueError(f"{path}: {name} is a finite number, got {value!r}")
    return float(value)
