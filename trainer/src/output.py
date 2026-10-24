"""The answer: the only thing the trainer sends back (synthetic-loop-dia.md, 18).

Every trainer, naive or learning, answers each pool the same way, so the
contract lives here rather than in any one trainer, and a malformed answer
fails here instead of reaching the synthetic Arduino.

The answer (trainer-plan.md T5): the command, an i32 whose sign is the
direction and magnitude the pulses per second, plus what comes next:
continue, reset the run in place for the next episode with that episode's
seed (synthetic-loop-dia.md, Reset in place), or stop the run. The
synthetic Arduino clamps the rate to +-56,666 pulses/s, so the trainer only
has to keep it an i32. On the trainer link it travels as its instance's
entry in one JSON line on the orchestrator's stdin, {"instance": k,
"command": c, "next": "continue" | "reset" | "stop"}, with "seed": S on a
reset only (synthetic-loop-dia.md, Contracts); on to the Arduino the
command alone travels, as 4 bytes little-endian.
"""

from dataclasses import dataclass

I32_MIN, I32_MAX = -(2**31), 2**31 - 1
SEED_LIMIT = 2**64  # the orchestrator reads the seed as a u64

CONTINUE, RESET, STOP = "continue", "reset", "stop"


@dataclass(frozen=True)
class Answer:
    command: int  # pulses per second, sign = direction; used on continue only
    next: str  # CONTINUE, RESET or STOP
    seed: int | None = None  # the next episode's seed, on a reset only

    def __post_init__(self) -> None:
        # bool is an int in Python, so it is refused explicitly.
        if isinstance(self.command, bool) or not isinstance(self.command, int):
            raise ValueError(f"the command is an integer, got {self.command!r}")
        if not I32_MIN <= self.command <= I32_MAX:
            raise ValueError(f"the command is an i32, got {self.command}")
        if self.next not in (CONTINUE, RESET, STOP):
            raise ValueError(f"next is continue, reset or stop, got {self.next!r}")
        if (self.next == RESET) != (self.seed is not None):
            raise ValueError(f"a reset carries the next episode's seed, and nothing else does: {self}")
        if self.seed is not None and (isinstance(self.seed, bool) or not isinstance(self.seed, int)
                                      or not 0 <= self.seed < SEED_LIMIT):
            raise ValueError(f"a seed is a whole number, 0 to 2**64 - 1, got {self.seed!r}")

    @staticmethod
    def go(command: int) -> "Answer":
        return Answer(command, CONTINUE)

    @staticmethod
    def reset(seed: int) -> "Answer":
        return Answer(0, RESET, seed)

    @staticmethod
    def stop() -> "Answer":
        return Answer(0, STOP)

    def entry(self) -> dict:
        """The answer's fields in its instance's entry on the link (18);
        the run adds "instance"."""
        message: dict = {"command": self.command, "next": self.next}
        if self.seed is not None:
            message["seed"] = self.seed
        return message
