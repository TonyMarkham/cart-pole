"""The capture record: every capture of a session, with the step it scored
(rl-rewrite-plan.md, R5: every scored step is recorded with its inputs).

One CSV per session in the run folder (run_folder.py), one row per capture
in the order they arrive. An episode's capture 0 has an empty `step`; every
other row is the capture that scored step `step`, with the action and
command of that step, its reward, the reward plugin's factors, the end
reason (empty when it went on) and whether the time limit truncated it.
The capture's MuJoCo time (time), the pool (p0-p29, as the link carries
it), the sites (left_x ... right_cart_z) and the pole state (pole_speed,
pole_angle, pole_com_x ... pole_hinge_z) are stored exactly (Python's
repr of each float), so the record replays to the same observations and
rewards (link/replay.py) and the measures read MuJoCo's truth from it
(measures/).
"""

import csv
from pathlib import Path

from contracts import FLOATS, SITE_ROLES, Capture

POOL_COLUMNS = [f"p{i}" for i in range(FLOATS)]
SITE_COLUMNS = [f"{role}_{axis}" for role in SITE_ROLES for axis in "xyz"]
POLE_COLUMNS = ["pole_speed", "pole_angle", *(f"pole_{point}_{axis}" for point in ("com", "hinge") for axis in "xyz")]
TIME_COLUMN = "time"
HEAD = ["instance", "episode", "id", TIME_COLUMN, "step", "action", "command"]
TAIL = ["reward", "end", "truncated"]


def header(factors: tuple[str, ...]) -> list[str]:
    return [*HEAD, *POOL_COLUMNS, *SITE_COLUMNS, *POLE_COLUMNS, *TAIL, *factors]


class Record:
    def __init__(self, path: Path, factors: tuple[str, ...]) -> None:
        self.path = path
        self.factors = factors
        self.file = open(path, "w", newline="", encoding="utf-8")
        self.rows = csv.writer(self.file)
        self.rows.writerow(header(factors))

    def capture(self, instance: int, episode: int, capture: Capture, scored: dict | None = None) -> None:
        """`scored`: the step this capture finished, None for capture 0:
        {"step", "action", "command", "reward", "factors", "end", "truncated"}."""
        if scored is None:
            head = [instance, episode, capture.id, repr(capture.time), "", "", ""]
            tail = ["", "", ""] + [""] * len(self.factors)
        else:
            head = [instance, episode, capture.id, repr(capture.time), scored["step"], repr(scored["action"]),
                    scored["command"]]
            tail = [repr(scored["reward"]), scored["end"] or "", int(scored["truncated"])]
            tail += [repr(scored["factors"][name]) for name in self.factors]
        self.rows.writerow([*head, *map(repr, capture.pool.flat()), *map(repr, capture.sites.flat()),
                            *map(repr, capture.pole.flat()), *tail])

    def flush(self) -> None:
        self.file.flush()

    def close(self) -> None:
        self.file.close()


def read(path: Path) -> list[dict]:
    """The rows as dicts of strings, in file order."""
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))
