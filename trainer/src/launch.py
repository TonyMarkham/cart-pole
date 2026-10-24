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
from typing import Protocol

from output import Answer
from pool import Pool
from settings import Settings


class Trainer(Protocol):
    def answer(self, pool: Pool) -> Answer: ...


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


def run_orchestrator(settings: Settings, trainer: Trainer) -> int:
    """Runs one run to its end and returns the orchestrator's exit code.

    Each pool is answered by `trainer`; after a stop the orchestrator shuts
    the run down and closes stdout, so no further pool may arrive. If the
    trainer stops first (an error, Ctrl+C), the orchestrator is stopped
    too: no run outlives the trainer that started it.
    """
    print(f"trainer: launching {' '.join(settings.orchestrator_command)}", flush=True)
    process = subprocess.Popen(
        settings.orchestrator_command,
        cwd=settings.orchestrator_cwd,
        env=orchestrator_environment(),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    )
    assert process.stdin is not None and process.stdout is not None
    try:
        stopped = False
        observations = 0
        for line in process.stdout:
            if stopped:
                raise ValueError(f"a pool arrived after the stop: {line!r}")
            answer = trainer.answer(read_pool(line, observations))
            observations += 1
            process.stdin.write(answer.line())
            process.stdin.flush()
            stopped = not answer.keep_going
        code = process.wait()
        print(f"trainer: {observations} pools answered, stop sent: {stopped}", flush=True)
        return code
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
