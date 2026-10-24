"""The answer: the only thing the trainer sends back (synthetic-loop-dia.md, 18).

Every trainer, naive or learning, answers each pool the same way, so the
contract lives here rather than in any one trainer, and a malformed answer
fails here instead of reaching the synthetic Arduino.

The answer (trainer-plan.md T5): the command, an i32 whose sign is the
direction and magnitude the pulses per second, plus continue or stop. The
synthetic Arduino clamps the rate to +-45,333 pulses/s, so the trainer only
has to keep it an i32. On to the Arduino the command alone travels, as 4
bytes little-endian (synthetic-loop-dia.md, Contracts); how the answer
itself travels to the orchestrator waits on the link (trainer-plan.md, Not
decided), so there is no encoding here yet.
"""

from dataclasses import dataclass

I32_MIN, I32_MAX = -(2**31), 2**31 - 1


@dataclass(frozen=True)
class Answer:
    command: int  # pulses per second, sign = direction
    keep_going: bool  # continue (True) or stop this run (False)

    def __post_init__(self) -> None:
        if not I32_MIN <= self.command <= I32_MAX:
            raise ValueError(f"the command is an i32, got {self.command}")
