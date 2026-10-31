"""The episode table: what the policy does with the pole, band by band.

The finished episodes (episodes.csv) are grouped by the run's step count
when each ended, into bands of `band_steps`; the pole's motion is MuJoCo's
pole angle and time in the capture record. The columns, as trainer-observations.md
defines them:

- Ends: episodes ended by an end plugin (not the time limit).
- Mean reward: the episode's total reward, averaged over the band.
- Full turns: (highest - lowest pole angle in the episode) / 2 pi,
  averaged over the band. MuJoCo's angle counts on past a full turn, so
  nothing is unwrapped.
- Spinning: episodes of `spin_turns` turns or more.
- Slow: episodes of fewer than `slow_turns` turns that no end plugin ended.
- Within: the share of the band's captures within `upright_degrees` of
  upright.
- Longest hold: the longest unbroken time within `upright_degrees` of
  upright, in any episode of the band, s.
- Slow visits: visits within `upright_degrees` of upright lasting
  `visit_seconds` or more; a visit is one stretch of captures there.
"""

import math

import checks
from measures.common import episodes, finished
from run_folder import RunFolder

KEYS = {"band_steps", "upright_degrees", "spin_turns", "slow_turns", "visit_seconds"}


def visits(captures, band: float) -> list[float]:
    """The durations, s, of each stretch of captures within `band` rad of upright."""
    stretches, current = [], None
    for c in captures:
        if abs(c.lean) <= band:
            current = (current or 0.0) + c.dt
        elif current is not None:
            stretches.append(current)
            current = None
    if current is not None:
        stretches.append(current)
    return stretches


def run(folder: RunFolder, table: dict) -> None:
    checks.only(table, KEYS, "episodes")
    width = checks.whole(table, "band_steps", "episodes.", 1)
    band = math.radians(checks.number(table, "upright_degrees", "episodes.", 0.0, 180.0))
    spin = checks.number(table, "spin_turns", "episodes.", 0.0)
    slow = checks.number(table, "slow_turns", "episodes.", 0.0)
    visit = checks.number(table, "visit_seconds", "episodes.", 0.0)
    ended, recorded = finished(folder), episodes(folder)
    bands: dict[int, list] = {}
    for number, row in sorted(ended.items()):
        if number in recorded:
            bands.setdefault(int(row["steps"]) // width, []).append((row, recorded[number]))
    print("| Steps | Episodes | Ends | Mean reward | Full turns | Spinning | Slow | Within | Longest hold | Slow visits |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for b, members in sorted(bands.items()):
        turns, within, total, holds, slow_visits, spinning, slow_n, ends = [], 0, 0, [0.0], 0, 0, 0, 0
        for row, caps in members:
            angles = [c.angle for c in caps]
            t = (max(angles) - min(angles)) / (2 * math.pi)
            turns.append(t)
            was_ended = row["ended"] != "time limit"
            ends += was_ended
            spinning += t >= spin
            slow_n += t < slow and not was_ended
            within += sum(abs(c.lean) <= band for c in caps)
            total += len(caps)
            stretches = visits(caps, band)
            holds.extend(stretches)
            slow_visits += sum(s >= visit for s in stretches)
        reward = sum(float(row["reward"]) for row, _ in members) / len(members)
        print(f"| {b * width:,d}–{(b + 1) * width:,d} | {len(members)} | {ends} | {reward:.1f} | "
              f"{sum(turns) / len(turns):.1f} | {spinning} | {slow_n} | {100 * within / total:.1f}% | "
              f"{max(holds):.2f} s | {slow_visits} |")
