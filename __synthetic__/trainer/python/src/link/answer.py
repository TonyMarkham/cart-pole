"""The answer: the only thing the trainer sends back (synthetic-loop-dia.md,
18; Contracts, the answer).

Continue with the command, reset in place with the next episode's seed
(Reset in place), or stop the whole run. On the link it is its instance's
entry, {"instance": k, "command": c, "next": ...}, with "seed": S on a
reset only.
"""

from dataclasses import dataclass

from contracts import Command

SEED_LIMIT = 2**64  # the orchestrator reads the seed as a u64

CONTINUE, RESET, STOP = "continue", "reset", "stop"


@dataclass(frozen=True)
class Answer:
    command: Command  # used on continue only
    next: str
    seed: int | None = None  # the next episode's seed, on a reset only

    def __post_init__(self) -> None:
        if not isinstance(self.command, Command):
            raise ValueError(f"answer: the command is a Command, got {self.command!r}")
        if self.next not in (CONTINUE, RESET, STOP):
            raise ValueError(f"answer: next is continue, reset or stop, got {self.next!r}")
        if (self.next == RESET) != (self.seed is not None):
            raise ValueError(f"answer: a reset carries the next episode's seed, and nothing else does: {self}")
        if self.seed is not None and (isinstance(self.seed, bool) or not isinstance(self.seed, int)
                                      or not 0 <= self.seed < SEED_LIMIT):
            raise ValueError(f"answer: a seed is a whole number, 0 to 2**64 - 1, got {self.seed!r}")

    @staticmethod
    def go(command: Command) -> "Answer":
        return Answer(command, CONTINUE)

    @staticmethod
    def reset(seed: int) -> "Answer":
        return Answer(Command(0), RESET, seed)

    @staticmethod
    def stop() -> "Answer":
        return Answer(Command(0), STOP)

    def entry(self) -> dict:
        """The answer's fields in its instance's entry (18); the link adds "instance"."""
        message: dict = {"command": self.command.rate, "next": self.next}
        if self.seed is not None:
            message["seed"] = self.seed
        return message
