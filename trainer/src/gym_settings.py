"""The trainer's Gymnasium settings, read from the file settings.toml's
[trainer] gym names (trainer/gym.toml).

The environment's own settings (trainer-plan.md T8) grow as the reward, the
end conditions and the learning method are decided, so they live in their
own file, apart from settings.toml's run settings.
"""

import tomllib
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class GymSettings:
    max_episode_steps: int  # [env]: the placeholder episode length, in observations

    @staticmethod
    def load(path: Path) -> "GymSettings":
        with open(path, "rb") as f:
            raw = tomllib.load(f)
        steps = raw["env"]["max_episode_steps"]
        # bool is an int in Python; a length of `true` is a typo, not 1.
        if not isinstance(steps, int) or isinstance(steps, bool) or steps < 1:
            raise ValueError(f"{path}: env.max_episode_steps is a whole number >= 1, got {steps!r}")
        return GymSettings(max_episode_steps=steps)
