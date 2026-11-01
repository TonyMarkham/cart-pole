"""The replay link: a recorded session played back through the trainer.

A strange episode can be re-driven through the environment, the plugins
and the policy without the loop (rl-rewrite-plan.md, 3.2). It reads a
session's capture record (record.py) and hands out its captures in order,
per instance, and answers ask() with the run's values from the manifest of
the run folder that holds the record. The driver answers with the recorded actions
(`recorded_action`); an answer the record does not hold (a continue where
the episode ended, a reset mid-episode, another command) means the replay
has diverged from the session, and fails it.
"""

from pathlib import Path

import record
from contracts import Capture, PoleState, Pool, RunValues, Sites
from link import RunFailed
from link.answer import CONTINUE, RESET, STOP, Answer


class ReplayLink:
    def __init__(self, path: Path) -> None:
        self.path = path
        rows = record.read(path)
        if not rows:
            raise ValueError(f"{path}: no captures recorded")
        self.instances = max(int(r["instance"]) for r in rows) + 1
        self.rows: list[list[dict]] = [[] for _ in range(self.instances)]
        for r in rows:
            self.rows[int(r["instance"])].append(r)
        self.cursor = [0] * self.instances  # each instance's next row
        self.advanced = list(range(self.instances))
        self.stopped = False

    def ask(self) -> RunValues:
        """The run's values the session ran with: the run's manifest records
        the orchestrator's answer (run_folder.py), and a resume never changes
        them."""
        from run_folder import RunFolder
        manifest = RunFolder(self.path.resolve().parent).read_manifest()
        if "run_values" not in manifest:
            raise ValueError(f"{self.path.parent}: the run's manifest records no run values")
        return RunValues.from_message(manifest["run_values"])

    def remaining(self, k: int) -> int:
        return len(self.rows[k]) - self.cursor[k]

    def recorded_action(self, k: int) -> float | None:
        """The action of instance k's next scored row; None when the next
        row starts a new episode, or the record ends."""
        if not self.remaining(k):
            return None
        row = self.rows[k][self.cursor[k]]
        return float(row["action"]) if row["step"] else None

    def next_captures(self) -> dict[int, Capture]:
        assert not self.stopped
        captures = {}
        for k in self.advanced:
            if not self.remaining(k):
                raise RunFailed(f"the record of instance {k} ends here")
            r = self.rows[k][self.cursor[k]]
            self.cursor[k] += 1
            captures[k] = Capture(int(r["id"]), float(r[record.TIME_COLUMN]),
                                  Pool.from_flat([r[c] for c in record.POOL_COLUMNS]),
                                  Sites.from_flat([r[c] for c in record.SITE_COLUMNS]),
                                  PoleState.from_flat([r[c] for c in record.POLE_COLUMNS]))
        return captures

    def answer(self, answers: dict[int, Answer]) -> None:
        assert not self.stopped
        if any(a.next == STOP for a in answers.values()):
            self.stopped = True
            return
        for k, a in answers.items():
            if not self.remaining(k):
                raise RunFailed(f"the record of instance {k} ends here")
            row = self.rows[k][self.cursor[k]]
            if a.next == RESET and row["step"]:
                raise RunFailed(f"instance {k}, id {row['id']}: the replay reset where the session went on")
            if a.next == CONTINUE and (not row["step"] or int(row["command"]) != a.command.rate):
                raise RunFailed(f"instance {k}, id {row['id']}: the replay answered {a.command.rate} where the "
                                f"session {'reset' if not row['step'] else 'answered ' + row['command']}")
        self.advanced = sorted(answers)

    def finish(self) -> int:
        assert self.stopped
        return 0

    def kill(self) -> None:
        pass
