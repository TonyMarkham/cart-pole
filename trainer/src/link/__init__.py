"""The trainer link behind one interface (rl-rewrite-plan.md, 3.2):

- `ask() -> RunValues`: the run's values (synthetic-loop-dia.md, Contracts,
  the run's values), called once, right after the link is opened and
  before anything else; the caller builds the run's facts from them;
- `next_captures() -> {instance: Capture}`: the advanced instances' next
  captures (17);
- `answer({instance: Answer})`: the answers to pending instances (18);
  their next captures come with the next `next_captures()`;
- `finish() -> exit code`: after a stop, the run closes (26);
- `kill()`: no run outlives the trainer that started it.

Three implementations: `real.py` (the orchestrator), `fake.py` (a scripted
rig for the tests) and `replay.py` (a recorded session played back).
"""

from typing import Protocol

from contracts import Capture, RunValues
from link.answer import Answer


class RunFailed(RuntimeError):
    """The run failed (26): training ends."""


class Link(Protocol):
    instances: int

    def ask(self) -> RunValues: ...

    def next_captures(self) -> dict[int, Capture]: ...

    def answer(self, answers: dict[int, Answer]) -> None: ...

    def finish(self) -> int: ...

    def kill(self) -> None: ...


def stop_unused(link: Link) -> None:
    """A link opened and asked but never driven (the gym file failed to load,
    or a resume was refused): its first captures are read and answered with
    stop, so the run closes in order (22-26) instead of being killed with
    its Orins and players running; killed only if that fails. Its own
    failure is not raised: the caller is already raising why the link went
    unused, and a run that cannot stop is broken anyway."""
    try:
        link.next_captures()
        link.answer({k: Answer.stop() for k in range(link.instances)})
        link.finish()
    except Exception:
        pass
    finally:
        link.kill()
