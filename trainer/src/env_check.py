"""The environment's check on the live loop: episodes of random actions,
back to back, on one instance.

Before anything learns, each episode must end and the next begin in place,
cleanly, or a learning failure would have two places to look. This drives
episodes of random actions to their end (an end plugin or the time limit)
and checks every observation against the observation space.

usage (from the repo root): uv run --project trainer python trainer/src/env_check.py [episodes]
"""

import sys

from env import LoopVecEnv
from link import stop_unused
from link.real import RealLink
from settings import Gym, Settings


def main() -> int:
    episodes = int(sys.argv[1]) if len(sys.argv) > 1 else 2
    settings = Settings.load()
    link = RealLink(settings, settings.seed, 1)
    try:
        gym = Gym.load(settings.gym, settings.facts(link.ask()))
    except BaseException:
        stop_unused(link)
        raise
    env = LoopVecEnv(gym, link, 1, settings.seed)
    try:
        observation = env.reset()
        for episode in range(episodes):
            steps, total, done = 0, 0.0, False
            while not done:
                assert env.observation_space.contains(observation[0]), observation[0]
                observation, rewards, dones, infos = env.step(env.action_space.sample()[None, :])
                steps, total, done = steps + 1, total + float(rewards[0]), bool(dones[0])
            print(f"env_check: episode {episode}: {steps} steps, ended by {infos[0]['end'] or 'the time limit'}, "
                  f"reward {total:.3f}, last pool {infos[0]['id']}", flush=True)
    finally:
        env.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
