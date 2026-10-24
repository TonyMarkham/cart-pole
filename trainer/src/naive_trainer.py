"""The naive trainer: bring-up's stand-in for the policy's teacher.

Before a policy can learn, the loop's plumbing has to be proven, and a
failure must have one place to look (trainer-plan.md, Bring-up). This
trainer learns nothing: it is a scripted agent on the trainer's Gymnasium
environment (loop_env.py), one episode, driving bring-up stage 2's slow
back-and-forth. That proves the command path (action -> i32 -> synthetic
Arduino -> cart speed -> MuJoCo -> render) and moves the markers across the
frame, so the run's truth log can be fitted to what the trainer sees
(observation_fit.py).

The action at step n is a = (peak / 56,666) x cos(2 pi n / period), so the
command is peak x cos(2 pi n / period) pulses/s; n, the step count, is the
only clock the trainer has. A cosine speed moves the cart in a sine centred
on its start; a sine speed would drift to one side. peak and period are
gym.toml's [sweep], and the episode ends at its [env] max_episode_steps
(TimeLimit), when the run is stopped and closed (22-26).

usage (from the repo root): uv run --project trainer python trainer/src/naive_trainer.py
"""

import math
import sys

import numpy as np

from gym_settings import MAX_RATE, GymSettings
from launch import RunFailed
from loop_env import make
from settings import Settings


def main() -> int:
    settings = Settings.load()
    gym_settings = GymSettings.load(settings.gym)
    sweep = gym_settings.sweep
    if sweep is None:
        raise ValueError(f"{settings.gym} has no [sweep]: the naive trainer's action")
    print(f"trainer: sweep from {settings.gym}: peak {sweep.peak} pulses/s, period {sweep.period}, "
          f"{gym_settings.max_episode_steps} steps", flush=True)
    env = make(settings, gym_settings)
    try:
        env.reset()
        steps, total, truncated, terminated = 0, 0.0, False, False
        while not (truncated or terminated):
            a = sweep.peak / MAX_RATE * math.cos(2 * math.pi * steps / sweep.period)
            _, reward, terminated, truncated, info = env.step(np.array([a], dtype=np.float32))
            steps += 1
            total += reward
        env.close()  # the stop (18), then the run closes (26), or RunFailed
    except RunFailed as error:
        print(f"trainer: the run failed: training ends ({error})", flush=True)
        return 1
    finally:
        env.close()
    ending = "the end crash" if terminated else "the time limit"
    print(f"trainer: {steps} steps, ended by {ending}, reward {total:.3f} "
          f"(mean {total / steps:.4f} a step), then the run closed", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
