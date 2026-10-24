"""The environment's check (trainer-plan.md Steps 2): runs back to back
through it, then Gymnasium's own check_env.

Before anything learns on the environment, it has to keep Gymnasium's
contract and close each run cleanly before the next, or a learning failure
would have two places to look. This drives two episodes of random actions to
their TimeLimit truncation (each reset() closes the open run and starts the
next), then runs check_env, skipping its render check (the environment
renders nothing). Each reset() is a whole run start, so the check takes
several runs' worth of startups.

usage (from the repo root): uv run --project trainer python trainer/src/env_check.py [episodes]
"""

import sys

from gymnasium.utils.env_checker import check_env

from loop_env import make


def main() -> int:
    episodes = int(sys.argv[1]) if len(sys.argv) > 1 else 2
    env = make()
    try:
        for episode in range(episodes):
            observation, info = env.reset()
            steps, truncated, terminated = 0, False, False
            while not (truncated or terminated):
                observation, reward, terminated, truncated, info = env.step(env.action_space.sample())
                steps += 1
                assert env.observation_space.contains(observation), observation
            print(f"env_check: episode {episode}: {steps} steps, truncated {truncated}, last pool {info['id']}", flush=True)
        check_env(env.unwrapped, skip_render_check=True)
        print("env_check: check_env passed", flush=True)
    finally:
        env.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
