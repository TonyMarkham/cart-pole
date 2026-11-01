"""The strike-gap end: synthetic-loop-dia.md, The episode end, the source
of truth.

  end if Left Cart.x - Left Rail.x < b
  end if Right Rail.x - Right Cart.x < b

The gaps are the cart's clearance to each base from the strike-face sites,
0 at contact, 0.440 m and 0.445 m with the cart centred. b is the safety
buffer, at least the most the cart moves from a check that passed to the
stop taking effect (from the run's timing and values). Checked on capture
t+1's sites, after r is paid; nothing is subtracted.
"""

from dataclasses import dataclass

import checks
from contracts import StepInputs
from plugins import RunFacts


def floor(facts: RunFacts) -> float:
    """b's least value, metres: the cart top speed times the time from a check
    that passed to the stop taking effect (synthetic-loop-dia.md, Numbers):
    the next capture is at most the longest gap later, and its stop takes
    effect at its arrival step, at most the longest latency after that. 52 +
    16 = 68 ms at 20-30 Hz, 8-15 ms and a 2 ms timestep."""
    return facts.cart_top_speed * (facts.longest_gap_ms() + facts.longest_latency_ms()) / 1000


@dataclass(frozen=True)
class StrikeGap:
    buffer: float  # b, metres

    @staticmethod
    def from_table(table: dict, facts: RunFacts, where: str) -> "StrikeGap":
        checks.only(table, {"buffer"}, where.rstrip("."))
        least = floor(facts)
        buffer = checks.number(table, "buffer", where, 0.0, None)
        if buffer < least:
            raise ValueError(
                f"{where}buffer: at least {least:.5f} m, the most the cart moves from a passing check to the stop "
                f"taking effect at the cart top speed {facts.cart_top_speed} m/s, command_rate_min "
                f"{facts.command_rate_min_hz} Hz, rnd_delay_max {facts.rnd_delay_max_ms} ms and the timestep "
                f"{facts.timestep} s (synthetic-loop-dia.md, The episode end), got {buffer}"
            )
        return StrikeGap(buffer=buffer)

    def check(self, inputs: StepInputs) -> str | None:
        s = inputs.current
        if s.left_cart[0] - s.left_rail[0] < self.buffer:
            return "left strike gap"
        if s.right_rail[0] - s.right_cart[0] < self.buffer:
            return "right strike gap"
        return None


PLUGIN = StrikeGap
