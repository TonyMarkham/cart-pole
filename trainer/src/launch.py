"""One run: the trainer launches the orchestrator and answers its pools.

The trainer owns the run (trainer-plan.md T1), so it is the process that
starts the orchestrator, with the command its settings.toml names
(synthetic-loop-dia.md, 1), and the one that learns how the run ended (26).

The trainer link (synthetic-loop-dia.md, Trainer link) is the orchestrator's
stdin and stdout, held here as pipes. The run holds one or more training
instances (synthetic-loop-dia.md, Training instances), so each line carries
a list: one stdout line, the pools of the instances advanced (17), and one
stdin line back, the answers to any pending instances (18), whose next pools
come in the next line. The orchestrator's log is on its stderr: the
trainer's console, or, given a log file, appended there, so a long training
run's console holds only its summaries. The run's end is the orchestrator's
exit: stdout closes, then exit code 0 (closed) or 1 (failed: training ends).
"""

import json
import os
import subprocess
from pathlib import Path

from output import STOP, Answer
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


def read_pools(line: str, expected_ids: dict[int, int]) -> dict[int, Pool]:
    """One stdout line (17): {"pools": [{"instance": k, "id": n, "pool": [6
    rows]}, ...]}, exactly the instances advanced, each at its next id.
    Anything else on stdout means the link is broken, so it fails the run."""
    try:
        message = json.loads(line)
    except json.JSONDecodeError as error:
        raise ValueError(f"the orchestrator's stdout is the link alone, got {line!r}") from error
    if not isinstance(message, dict) or set(message) != {"pools"} or not isinstance(message["pools"], list):
        raise ValueError(f"a pools message is {{'pools': [...]}}, got {line!r}")
    pools = {}
    for entry in message["pools"]:
        if not isinstance(entry, dict) or set(entry) != {"instance", "id", "pool"}:
            raise ValueError(f"a pool entry is {{'instance', 'id', 'pool'}}, got {entry!r}")
        k = entry["instance"]
        if k not in expected_ids or k in pools:
            raise ValueError(f"pool for instance {k!r}, expected each of {sorted(expected_ids)} once")
        if entry["id"] != expected_ids[k]:
            raise ValueError(f"instance {k}: expected pool {expected_ids[k]}, got pool {entry['id']!r}")
        pools[k] = Pool.from_rows(entry["pool"])
    if set(pools) != set(expected_ids):
        raise ValueError(f"expected pools for instances {sorted(expected_ids)}, got {sorted(pools)}")
    return pools


class Run:
    """One run, from the launch (1) to the exit (26).

    The link alternates: `next_pools()` reads the pools of the instances
    advanced, `answer()` answers any of the pending ones, and the next
    `next_pools()` reads exactly those instances' next pools. After a stop,
    `finish()` waits for the run to close. A run that ends without a stop,
    or with exit code 1, raises RunFailed. If anything else goes wrong,
    `kill()` makes sure no run outlives the trainer that started it.
    """

    def __init__(
        self, settings: Settings, seed: int | None = None, log: Path | None = None, instances: int | None = None
    ) -> None:
        self.instances = settings.instances if instances is None else instances
        command = settings.command(seed, self.instances)
        self.log = log
        self.log_file = open(log, "a", encoding="utf-8") if log is not None else None
        launching = f"trainer: launching {' '.join(command)}"
        if self.log_file is not None:
            self.log_file.write(launching + "\n")
            self.log_file.flush()
        else:
            print(launching, flush=True)
        self.process = subprocess.Popen(
            command,
            cwd=settings.orchestrator_cwd,
            env=orchestrator_environment(),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.log_file,
            encoding="utf-8",
        )
        self.next_ids = [0] * self.instances  # per instance, the next pool's id
        self.advanced = list(range(self.instances))  # whose pools the next line holds
        self.stopped = False

    def failed(self, what: str) -> RunFailed:
        where = f" (the run's log: {self.log})" if self.log is not None else ""
        return RunFailed(what + where)

    def next_pools(self) -> dict[int, Pool]:
        """The advanced instances' pools (17), by instance. The run ending
        instead means it failed."""
        assert self.process.stdout is not None and not self.stopped
        line = self.process.stdout.readline()
        if not line:
            raise self.failed(f"the run ended before the next pools: exit code {self.process.wait()}")
        pools = read_pools(line, {k: self.next_ids[k] for k in self.advanced})
        for k in pools:
            self.next_ids[k] += 1
        return pools

    def answer(self, answers: dict[int, Answer]) -> None:
        """The answers to pending instances (18), by instance; their next
        pools come in the next line. A stop answers every instance."""
        assert self.process.stdin is not None and not self.stopped
        entries = [{"instance": k, **answers[k].entry()} for k in sorted(answers)]
        self.process.stdin.write(json.dumps({"answers": entries}) + "\n")
        self.process.stdin.flush()
        self.advanced = sorted(answers)
        self.stopped = any(answer.next == STOP for answer in answers.values())

    def finish(self) -> int:
        """After the stop: the run shuts down (22-25) and closes stdout;
        no further pool may arrive. Returns the exit code (26)."""
        assert self.process.stdout is not None and self.stopped
        extra = self.process.stdout.read()
        if extra:
            raise ValueError(f"output arrived after the stop: {extra!r}")
        code = self.process.wait()
        if code != CLOSED:
            raise self.failed(f"the run did not close after the stop: exit code {code}")
        return code

    def kill(self) -> None:
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        if self.log_file is not None:
            self.log_file.close()
