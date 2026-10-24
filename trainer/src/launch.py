"""One run: the trainer launches the orchestrator and answers its pools.

The trainer owns the run (trainer-plan.md T1), so it is the process that
starts the orchestrator, with the command its settings.toml names
(synthetic-loop-dia.md, 1), and the one that learns how the run ended (26).

The trainer link (synthetic-loop-dia.md, Trainer link) is the orchestrator's
stdin and stdout, held here as pipes: per observation one stdout line, the
pool (17), and one stdin line back, the answer (18). The orchestrator's log
is on its stderr, which stays the trainer's console. The run's end is the
orchestrator's exit: stdout closes, then exit code 0 (closed) or 1 (failed:
training ends).
"""

import json
import os
import subprocess

from output import Answer
from pool import Pool
from settings import Settings

CLOSED, FAILED = 0, 1  # the orchestrator's exit codes (26)


class RunFailed(RuntimeError):
    """The run failed (26): training ends."""


def orchestrator_environment() -> dict[str, str]:
    """The trainer's environment, minus its own virtual environment.

    `uv run` sets VIRTUAL_ENV to trainer/.venv for the trainer's Python,
    and a child inherits it; the orchestrator's own `uv run --project orin`
    then warns that it does not match orin/.venv. The orchestrator is not
    the trainer's Python, so it does not get the trainer's venv.
    """
    environment = dict(os.environ)
    environment.pop("VIRTUAL_ENV", None)
    return environment


def read_pool(line: str, expected_id: int) -> Pool:
    """One stdout line (17): {"id": n, "pool": [6 rows]}. Anything else on
    stdout means the link is broken, so it fails the run."""
    try:
        message = json.loads(line)
    except json.JSONDecodeError as error:
        raise ValueError(f"the orchestrator's stdout is the link alone, got {line!r}") from error
    if not isinstance(message, dict) or set(message) != {"id", "pool"}:
        raise ValueError(f"a pool message is {{'id', 'pool'}}, got {line!r}")
    if message["id"] != expected_id:
        raise ValueError(f"expected pool {expected_id}, got pool {message['id']!r}")
    return Pool.from_rows(message["pool"])


class Run:
    """One run, from the launch (1) to the exit (26).

    The link alternates: `next_pool()` reads pool n, `answer()` sends the
    answer to it, and so on. After a stop, `finish()` waits for the run to
    close. A run that ends without a stop, or with exit code 1, raises
    RunFailed. If anything else goes wrong, `kill()` makes sure no run
    outlives the trainer that started it.
    """

    def __init__(self, settings: Settings, seed: int | None = None) -> None:
        command = settings.command(seed)
        print(f"trainer: launching {' '.join(command)}", flush=True)
        self.process = subprocess.Popen(
            command,
            cwd=settings.orchestrator_cwd,
            env=orchestrator_environment(),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            encoding="utf-8",
        )
        self.observations = 0  # pools read so far; the next one's id
        self.stopped = False

    def next_pool(self) -> Pool:
        """Pool n (17). The run ending instead means it failed."""
        assert self.process.stdout is not None and not self.stopped
        line = self.process.stdout.readline()
        if not line:
            raise RunFailed(f"the run ended before pool {self.observations}: exit code {self.process.wait()}")
        pool = read_pool(line, self.observations)
        self.observations += 1
        return pool

    def answer(self, answer: Answer) -> None:
        """The answer to the last pool (18)."""
        assert self.process.stdin is not None and not self.stopped
        self.process.stdin.write(answer.line())
        self.process.stdin.flush()
        self.stopped = not answer.keep_going

    def finish(self) -> int:
        """After the stop: the run shuts down (22-25) and closes stdout;
        no further pool may arrive. Returns the exit code (26)."""
        assert self.process.stdout is not None and self.stopped
        extra = self.process.stdout.read()
        if extra:
            raise ValueError(f"output arrived after the stop: {extra!r}")
        code = self.process.wait()
        if code != CLOSED:
            raise RunFailed(f"the run did not close after the stop: exit code {code}")
        return code

    def kill(self) -> None:
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
