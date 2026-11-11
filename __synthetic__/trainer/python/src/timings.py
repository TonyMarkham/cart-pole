"""The random timings' bands and the loss: settings.toml [orchestrator]
(__documentation__/synthetic-loop-dia.md, Timing; user, 2026-11-08)."""

from dataclasses import dataclass

import checks

# Each band's min and max, ms, in the order the orchestrator picks them, each
# passed as the flag of the same name with dashes (frame_ready_min as
# --frame-ready-min).
BANDS = ("frame_ready", "to_uno", "change_motor", "next_frame_ready")


@dataclass(frozen=True)
class Timings:
    """Stretch 1 (frame_ready_*), stretch 2 (to_uno_*), stretch 3
    (change_motor_*), the wait (next_frame_ready_*), ms, and
    drop_one_message_in. The orchestrator picks every step's timings from
    them at each reset (the schedule) and refuses bands that break the walk's
    orders; here each band is 0 <= min <= max and the loss a whole number of
    at least 1."""

    bands: dict  # name -> (min, max), ms
    drop_one_message_in: int

    def arguments(self) -> list[str]:
        flags = []
        for name in BANDS:
            low, high = self.bands[name]
            dashed = name.replace("_", "-")
            flags += [f"--{dashed}-min", str(low), f"--{dashed}-max", str(high)]
        return [*flags, "--drop-one-message-in", str(self.drop_one_message_in)]

    def longest_capture_gap_ms(self) -> float:
        """The most two captures can be apart before rounding: this step's
        stretch 2 and wait, and its picture's stretch 1 against the next's,
        frame_ready_max + to_uno_max + next_frame_ready_max - frame_ready_min
        (48 + 27.8 + 16.7 - 43 = 49.5 ms today; __documentation__/synthetic-loop-dia.md, The
        schedule). Every pick is below its band's max, so no gap reaches it."""
        b = self.bands
        return b["frame_ready"][1] + b["to_uno"][1] + b["next_frame_ready"][1] - b["frame_ready"][0]

    def shortest_capture_gap_ms(self) -> float:
        """The least two captures can be apart before rounding:
        frame_ready_min + to_uno_min + next_frame_ready_min - frame_ready_max
        (43 + 20.5 + 0 - 48 = 15.5 ms today)."""
        b = self.bands
        return b["frame_ready"][0] + b["to_uno"][0] + b["next_frame_ready"][0] - b["frame_ready"][1]

    @staticmethod
    def from_table(o: dict) -> "Timings":
        bands = {}
        for name in BANDS:
            low = checks.number(o, f"{name}_min", "orchestrator.", 0.0)
            high = checks.number(o, f"{name}_max", "orchestrator.", 0.0)
            if low > high:
                raise ValueError(f"orchestrator.{name}_min {low} is above {name}_max {high}")
            bands[name] = (low, high)
        return Timings(bands, checks.whole(o, "drop_one_message_in", "orchestrator.", 1))
