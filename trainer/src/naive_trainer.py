"""The naive trainer: bring-up's stand-in for the policy's teacher.

Before a policy can learn, the loop's plumbing has to be proven, and a
failure must have one place to look (trainer-plan.md, Bring-up). This
trainer learns nothing: it answers every pool with one fixed command and
continue, then stop after a set number of observations (trainer-plan.md,
Steps 1). Command 0 is bring-up stage 1 - the cart holds still and the pole
hangs or swings. Its input is pool.py's contract, its output output.py's.

Run as a script, it starts one run (1): it launches the orchestrator with
trainer/settings.toml's command and waits for it to exit.

Not here yet, each waiting on the diagram (trainer-plan.md, Not decided):
- the link to the orchestrator (17, 18, 26), so no pool reaches answer()
  yet and the run's end is only the orchestrator's exit code;
- scoring, since the reward and the end crash's bounds are not set.

usage (from the repo root): uv run --project trainer python trainer/src/naive_trainer.py
"""

import sys

from launch import run_orchestrator
from output import Answer
from pool import Pool
from settings import Settings


class NaiveTrainer:
    def __init__(self, command: int = 0, observations: int = 250) -> None:
        if observations < 1:
            raise ValueError(f"a run takes at least 1 observation, got {observations}")
        self.command = command
        self.observations = observations  # the answer to this many-th pool is stop
        self.seen = 0

    def answer(self, pool: Pool) -> Answer:
        self.seen += 1
        return Answer(self.command, keep_going=self.seen < self.observations)


def main() -> int:
    code = run_orchestrator(Settings.load())
    print(f"trainer: the orchestrator exited with code {code}", flush=True)
    return code


if __name__ == "__main__":
    sys.exit(main())
