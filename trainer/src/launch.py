"""Starting the run: the trainer launches the orchestrator (synthetic-loop-dia.md, 1).

The trainer owns the run (trainer-plan.md T1), so it is the process that
starts the orchestrator, with the command its settings.toml names, and
the one that knows when the run's process has ended. The orchestrator's
output goes to the trainer's own console.

What the orchestrator reports back (26) waits on the trainer's link to it
(trainer-plan.md, Not decided); until then the trainer sees only that the
process exited, and with which code.
"""

import os
import subprocess

from settings import Settings


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


def run_orchestrator(settings: Settings) -> int:
    """Launches the orchestrator, waits for it to exit, returns its exit code.

    If the trainer stops first (an error, Ctrl+C), the orchestrator is
    stopped too: no run outlives the trainer that started it.
    """
    print(f"trainer: launching {' '.join(settings.orchestrator_command)}", flush=True)
    process = subprocess.Popen(
        settings.orchestrator_command,
        cwd=settings.orchestrator_cwd,
        env=orchestrator_environment(),
    )
    try:
        return process.wait()
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
