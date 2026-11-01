"""What the measures share: the run's captures grouped by episode, with the
pole's angle and the capture's time as MuJoCo gave them, from the capture
record (record.py)."""

import csv
import math
from dataclasses import dataclass

import record
from run_folder import RunFolder


@dataclass(frozen=True)
class Capture:
    angle: float  # rad: MuJoCo's pole joint qpos, 0 hanging, pi upright, counting on past a full turn
    lean: float  # rad: the angle from straight up, 0 upright, +-pi hanging
    dt: float  # s: MuJoCo's time since the episode's previous capture (0 for capture 0)
    reward: float | None  # None for capture 0
    end: str  # the end reason, "" when the episode went on


def episodes(folder: RunFolder) -> dict[int, list[Capture]]:
    """Every recorded episode's captures in order, by episode number, over
    all the run's sessions."""
    found: dict[int, list[Capture]] = {}
    last_time: dict[int, float] = {}
    for path in folder.sessions():
        for row in record.read(path):
            scored = bool(row["step"])
            number = int(row["episode"])
            angle, time = float(row["pole_angle"]), float(row[record.TIME_COLUMN])
            found.setdefault(number, []).append(Capture(
                angle=angle,
                lean=math.remainder(angle - math.pi, 2 * math.pi),
                dt=time - last_time[number] if scored else 0.0,
                reward=float(row["reward"]) if scored else None,
                end=row["end"],
            ))
            last_time[number] = time
    return found


def finished(folder: RunFolder) -> dict[int, dict]:
    """episodes.csv's rows by episode number: the episodes that finished."""
    if not folder.episodes.is_file():
        return {}
    with open(folder.episodes, newline="", encoding="utf-8") as f:
        return {int(row["episode"]): row for row in csv.DictReader(f)}
