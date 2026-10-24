"""The environment's check (trainer-plan.md Steps 2): runs back to back
through it.

Before anything learns on the environment, it has to close each run cleanly
before the next, or a learning failure would have two places to look. This
drives two episodes of random actions to their end, the TimeLimit or the
end crash (each reset() closes the open run and starts the next), and checks
every observation against the observation space. Gymnasium's own check_env
is not run here (user, 2026-10-24): its short runs test the API, not motion.

usage (from the repo root): uv run --project trainer python trainer/src/env_check.py [episodes]
"""

import sys

from loop_env import make


def main() -> int:
    episodes = int(sys.argv[1]) if len(sys.argv) > 1 else 2
    env = make()
    try:
        for episode in range(episodes):
            observation, info = env.reset()
            steps, total, truncated, terminated = 0, 0.0, False, False
            while not (truncated or terminated):
                observation, reward, terminated, truncated, info = env.step(env.action_space.sample())
                steps += 1
                total += reward
                assert env.observation_space.contains(observation), observation
            print(f"env_check: episode {episode}: {steps} steps, truncated {truncated}, terminated {terminated}, "
                  f"reward {total:.3f}, last pool {info['id']}", flush=True)
    finally:
        env.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
