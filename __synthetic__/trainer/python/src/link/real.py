"""The real link: the trainer launches the orchestrator and speaks the lines.

The trainer owns the run, so it starts the orchestrator with the command
settings.toml names (synthetic-loop-dia.md, 1) and learns how the run ended
(26). The link is the orchestrator's stdin and stdout, held here as pipes
(The trainer link): first, once, the request for the run's values and its
answer (Contracts, the run's values); then one stdout line holds the
advanced instances' captures (17), one stdin line answers any pending
instances (18). The orchestrator's
log is on its stderr: the console, or appended to a log file. A line that
breaks the contract fails the run here (rl-rewrite-plan.md, R4).
"""

import json
import os
import subprocess
from pathlib import Path

from contracts import Capture, PoleState, Pool, RunValues, Sites
from link import RunFailed
from link.answer import STOP, Answer
from settings import Settings

CLOSED = 0  # the orchestrator's exit code for a closed run (26); 1 is failed

REQUEST = {"request": "run"}  # the trainer's first stdin line (Contracts, the run's values)


def orchestrator_environment() -> dict[str, str]:
    """The trainer's environment minus its own virtual environment: `uv run`
    sets VIRTUAL_ENV for the trainer, and the orchestrator's `uv run
    --project __synthetic__/orin/python` would warn that it does not match __synthetic__/orin/python/.venv."""
    environment = dict(os.environ)
    environment.pop("VIRTUAL_ENV", None)
    return environment


def read_run(line: str) -> RunValues:
    """The orchestrator's first stdout line, the answer to the request:
    {"run": {...}}, the seven run's values (contracts.RUN_KEYS)."""
    try:
        message = json.loads(line)
    except json.JSONDecodeError as error:
        raise ValueError(f"the orchestrator's first line answers the request for the run's values, got {line!r}") \
            from error
    if not isinstance(message, dict) or set(message) != {"run"}:
        raise ValueError(f"the answer to the request is {{'run': {{...}}}}, got {line!r}")
    return RunValues.from_message(message["run"])


def read_captures(line: str, expected_ids: dict[int, int]) -> dict[int, Capture]:
    """One stdout line (17): {"pools": [{"instance": k, "id": n, "pool": [6
    rows], "sites": {...}}, ...]}, exactly the advanced instances, each at its
    next id. Anything else on stdout means the link is broken."""
    try:
        message = json.loads(line)
    except json.JSONDecodeError as error:
        raise ValueError(f"the orchestrator's stdout is the link alone, got {line!r}") from error
    if not isinstance(message, dict) or set(message) != {"pools"} or not isinstance(message["pools"], list):
        raise ValueError(f"a pools line is {{'pools': [...]}}, got {line!r}")
    captures = {}
    for entry in message["pools"]:
        if not isinstance(entry, dict) or set(entry) != {"instance", "id", "time", "pool", "sites", "pole"}:
            raise ValueError(f"a pool entry is {{'instance', 'id', 'time', 'pool', 'sites', 'pole'}}, got {entry!r}")
        k = entry["instance"]
        if k not in expected_ids or k in captures:
            raise ValueError(f"pool for instance {k!r}, expected each of {sorted(expected_ids)} once")
        if entry["id"] != expected_ids[k]:
            raise ValueError(f"instance {k}: expected pool {expected_ids[k]}, got pool {entry['id']!r}")
        captures[k] = Capture(entry["id"], entry["time"], Pool.from_rows(entry["pool"]),
                              Sites.from_message(entry["sites"]),
                              PoleState.from_message(entry["pole"]))
    if set(captures) != set(expected_ids):
        raise ValueError(f"expected pools for instances {sorted(expected_ids)}, got {sorted(captures)}")
    return captures


class RealLink:
    def __init__(self, settings: Settings, seed: int, instances: int, log: Path | None = None,
                 out: Path | None = None) -> None:
        settings.require_built()
        self.instances = instances
        command = settings.command(seed, instances, out)
        self.log = log
        self.log_file = open(log, "a", encoding="utf-8") if log is not None else None
        launching = f"trainer: launching {' '.join(command)}"
        if self.log_file is not None:
            self.log_file.write(launching + "\n")
            self.log_file.flush()
        else:
            print(launching, flush=True)
        self.process = subprocess.Popen(
            command, cwd=settings.orchestrator_cwd, env=orchestrator_environment(),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log_file, encoding="utf-8",
        )
        self.next_ids = [0] * instances
        self.advanced = list(range(instances))  # whose captures the next line holds
        self.stopped = False
        self.values: RunValues | None = None  # the answer to ask(), kept for the run

    def failed(self, what: str) -> RunFailed:
        return RunFailed(what + (f" (the run's log: {self.log})" if self.log is not None else ""))

    def ask(self) -> RunValues:
        """The request, then its answer: the orchestrator answers once it has
        launched and registered every instance, before the first pools."""
        assert self.process.stdin is not None and self.process.stdout is not None and self.values is None
        try:
            self.process.stdin.write(json.dumps(REQUEST) + "\n")
            self.process.stdin.flush()
        except OSError as error:
            raise self.failed(f"the run ended before the request for its values: {error}") from error
        line = self.process.stdout.readline()
        if not line:
            raise self.failed(f"the run ended before answering the request for its values: "
                              f"exit code {self.process.wait()}")
        self.values = read_run(line)
        return self.values

    def next_captures(self) -> dict[int, Capture]:
        assert self.process.stdout is not None and not self.stopped
        assert self.values is not None, "ask() comes first: the orchestrator answers it before any pools"
        line = self.process.stdout.readline()
        if not line:
            raise self.failed(f"the run ended before the next pools: exit code {self.process.wait()}")
        captures = read_captures(line, {k: self.next_ids[k] for k in self.advanced})
        for k in captures:
            self.next_ids[k] += 1
        return captures

    def answer(self, answers: dict[int, Answer]) -> None:
        assert self.process.stdin is not None and not self.stopped
        entries = [{"instance": k, **answers[k].entry()} for k in sorted(answers)]
        self.process.stdin.write(json.dumps({"answers": entries}) + "\n")
        self.process.stdin.flush()
        self.advanced = sorted(answers)
        self.stopped = any(a.next == STOP for a in answers.values())

    def finish(self) -> int:
        """After the stop: the run shuts down (22-25) and closes stdout."""
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
            self.log_file = None
