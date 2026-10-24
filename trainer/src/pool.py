"""The pool: the only thing the trainer is given (synthetic-loop-dia.md, 17).

The policy may only learn from what the robot sees, so the trainer's whole
input is the Orin's last six marker frames, never MuJoCo's truth (trainer-plan.md
T2). This module holds that contract and checks it, so a malformed pool
fails here instead of being scored.

The pool (synthetic-loop-dia.md, Contracts): 6 entries, newest first, each
keyed by its age in -ms from the current capture time (the newest is 0), each
holding the 4 marker (x, y) in class order, normalized to [0, 1], or (-1, -1)
where that class was not detected. Flattened, 6 x (1 + 4 x 2) = 54 floats.
"""

from dataclasses import dataclass

ENTRIES = 6
MARKERS = 4

# Slot order = the repo's class ids (AGENTS.md, Verified contracts).
LEFT, RIGHT, PIVOT, END = range(MARKERS)

NOT_DETECTED = (-1.0, -1.0)

FLOATS = ENTRIES * (1 + MARKERS * 2)  # 54


@dataclass(frozen=True)
class Entry:
    age_ms: float  # -ms from the current capture time; the newest is 0
    markers: tuple[tuple[float, float], ...]  # 4 slots, class order

    def detected(self, slot: int) -> bool:
        return self.markers[slot] != NOT_DETECTED


@dataclass(frozen=True)
class Pool:
    entries: tuple[Entry, ...]  # newest first

    def __post_init__(self) -> None:
        if len(self.entries) != ENTRIES:
            raise ValueError(f"a pool holds {ENTRIES} entries, got {len(self.entries)}")
        if self.entries[0].age_ms != 0:
            raise ValueError(f"the newest entry's age is 0, got {self.entries[0].age_ms}")
        ages = [entry.age_ms for entry in self.entries]
        if any(older >= newer for newer, older in zip(ages, ages[1:])):
            raise ValueError(f"ages run newest first and fall strictly: {ages}")
        for entry in self.entries:
            if len(entry.markers) != MARKERS:
                raise ValueError(f"an entry holds {MARKERS} markers, got {len(entry.markers)}")
            for x, y in entry.markers:
                # -1 lies outside [0, 1], so "not detected" is never a position.
                if (x, y) != NOT_DETECTED and not (0 <= x <= 1 and 0 <= y <= 1):
                    raise ValueError(f"a marker is in [0, 1] or (-1, -1), got ({x}, {y})")

    @property
    def newest(self) -> Entry:
        return self.entries[0]

    def flat(self) -> list[float]:
        """The policy's input: per entry, the age then x0, y0, ... x3, y3."""
        return [v for e in self.entries for v in (e.age_ms, *(c for m in e.markers for c in m))]

    @staticmethod
    def from_flat(values: list[float]) -> "Pool":
        if len(values) != FLOATS:
            raise ValueError(f"a flat pool is {FLOATS} floats, got {len(values)}")
        width = FLOATS // ENTRIES
        rows = [values[i : i + width] for i in range(0, FLOATS, width)]
        return Pool(
            tuple(
                Entry(row[0], tuple((row[1 + 2 * m], row[2 + 2 * m]) for m in range(MARKERS)))
                for row in rows
            )
        )
