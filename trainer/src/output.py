"""The answer: the only thing the trainer sends back (synthetic-loop-dia.md, 18).

Every trainer, naive or learning, answers each pool the same way, so the
contract lives here rather than in any one trainer, and a malformed answer
fails here instead of reaching the synthetic Arduino.

The answer (trainer-plan.md T5): the command, an i32 whose sign is the
direction and magnitude the pulses per second, plus continue or stop. The
synthetic Arduino clamps the rate to +-56,666 pulses/s, so the trainer only
has to keep it an i32. On the trainer link it travels as one JSON line on
the orchestrator's stdin, {"command": c, "continue": true|false}
(synthetic-loop-dia.md, Contracts); on to the Arduino the command alone
travels, as 4 bytes little-endian.
"""

import json
from dataclasses import dataclass

I32_MIN, I32_MAX = -(2**31), 2**31 - 1


@dataclass(frozen=True)
class Answer:
    command: int  # pulses per second, sign = direction
    keep_going: bool  # continue (True) or stop this run (False)

    def __post_init__(self) -> None:
        # bool is an int in Python, so it is refused explicitly.
        if isinstance(self.command, bool) or not isinstance(self.command, int):
            raise ValueError(f"the command is an integer, got {self.command!r}")
        if not I32_MIN <= self.command <= I32_MAX:
            raise ValueError(f"the command is an i32, got {self.command}")

    def line(self) -> str:
        """The answer as the link carries it (18), newline included."""
        return json.dumps({"command": self.command, "continue": self.keep_going}) + "\n"
