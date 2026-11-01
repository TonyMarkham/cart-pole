"""The episode tracker: the one place a step's inputs are assembled.

Per instance it keeps the previous capture, the command history and the
step count, and turns each new capture into the `StepInputs` of the step
just finished (rl-rewrite-plan.md, 3.2). The first-step rule is
synthetic-loop-dia.md's (The reward): capture 0, the episode's first observation,
is not scored; the first scored step is c_0 with capture 1, taking capture
0's sites and pole state as the previous ones, and c_(-1) = 0, the stepper
at rest after the reset.
"""

from contracts import Capture, Command, StepInputs
from plugins import History


class Episode:
    def __init__(self, history_length: int) -> None:
        # At least one past command: the reward reads c_(t-1).
        self.history_length = max(history_length, 1)
        self.previous: Capture | None = None
        self.history = History.empty(self.history_length)
        self.steps = 0  # scored steps so far

    def start(self, capture: Capture) -> None:
        """Capture 0 of a new episode: kept, never scored."""
        self.previous = capture
        self.history = History.empty(self.history_length)
        self.steps = 0

    def advance(self, action: float, command: Command, capture: Capture) -> StepInputs:
        """Command c_t was answered with `action` and the next capture is in:
        the inputs of step t."""
        if self.previous is None:
            raise RuntimeError("advance() before the episode's first capture")
        inputs = StepInputs(
            previous=self.previous.sites,
            current=capture.sites,
            command=command.rate,
            previous_command=self.history.commands[0],
            step=self.steps,
            previous_pole=self.previous.pole,
            current_pole=capture.pole,
        )
        self.history = self.history.push(action, command.rate)
        self.previous = capture
        self.steps += 1
        return inputs
