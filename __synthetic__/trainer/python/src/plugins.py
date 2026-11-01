"""The four plugin roles and how a plugin is found by name
(rl-rewrite-plan.md, 3.1).

The reward, the episode end, the observation and the action are each
chosen and configured in the gym file, so changing one is a new file and
a new name there, never an edit to its callers. Every plugin follows one
pattern:

- one file in its role's folder (`rewards/`, `endings/`, `observations/`,
  `actions/`), named as the toml names it, holding one class and
  `PLUGIN = <that class>`;
- `from_table(table, facts)`, which reads and checks the plugin's own
  table (`[end.<name>]` and so on) and nothing else;
- the one method its role needs (the protocols below).

A reward scheme is the exception (user, 2026-10-29): its values sit one
table per factor, `[reward.small_control]`, `[reward.centered]` and so on,
which any scheme using that factor shares. The scheme names its factors in
`TABLES`, and `from_table` gets exactly those tables, keyed by name.

A plugin file goes when no run that can still resume names it (user,
2026-10-28: the reward plugins before tip_energy went with the runs that
lacked MuJoCo's pole state; 2026-10-29: tip_energy and tip_energy_cart
went with the nine-site runs, whose stored steps the twelve sites no
longer read; 2026-11-01: tip_energy_cart_centre and tip_speed_cart_centre
went once dm_control_cartpole was the scheme in use).
"""

import importlib
import math
import pkgutil
from dataclasses import dataclass, field
from typing import Any, Protocol

import numpy as np
from gymnasium import spaces

from contracts import Command, Pool, StepInputs

FOLDERS = {"observation": "observations", "action": "actions", "reward": "rewards", "end": "endings"}


@dataclass(frozen=True)
class RunFacts:
    """What a plugin may need to know about the run beyond its own table:
    the timing bands the toml files own, and the run's values the orchestrator
    answers with (synthetic-loop-dia.md, Contracts, the run's values)."""

    rnd_delay_max_ms: float  # [orchestrator] rnd_delay_max: the longest latency
    command_rate_min_hz: float  # the gym file's [env] command_rate_min: the longest gap
    cart_top_speed: float  # m/s: the MJCF actuator's ctrlrange
    top_pulse_rate: int  # pulses/s: the command's bound, the synthetic Arduino's clamp
    timestep: float  # s: MuJoCo's, the grid captures sit on
    pole_mass: float  # kg: everything that swings on the pole joint
    pole_com_distance: float  # m: its centre of mass's distance from the joint's axis
    pole_inertia: float  # kg m^2: its moment of inertia about that axis
    gravity: float  # m/s^2: the size of the model's gravity, straight down

    def first_step_after(self, ms: float) -> float:
        """The time, ms, of the first MuJoCo step strictly later than `ms`
        after a step: how the loop rounds a time to its grid
        (synthetic-loop-dia.md): 16 ms for 15 ms and a 2 ms timestep, and 52
        ms for 50 ms."""
        step_ms = self.timestep * 1000
        return step_ms * math.floor(ms / step_ms) + step_ms

    def longest_latency_ms(self) -> float:
        """The most from a capture to its arrival step."""
        return self.first_step_after(self.rnd_delay_max_ms)

    def longest_gap_ms(self) -> float:
        """The most between two captures: the slowest command rate's gap."""
        return self.first_step_after(1000 / self.command_rate_min_hz)


@dataclass(frozen=True)
class History:
    """An instance's last commands this episode, newest first, zero-filled
    to `length` at the episode's start (the reset leaves the cart at rest:
    command 0, synthetic-loop-dia.md, The policy sees its last commands)."""

    actions: tuple[float, ...]  # the policy's actions a, clipped to [-1, 1]
    commands: tuple[int, ...]  # the commands sent, pulses/s

    @staticmethod
    def empty(length: int) -> "History":
        return History((0.0,) * length, (0,) * length)

    def push(self, action: float, command: int) -> "History":
        n = len(self.actions)
        return History((action, *self.actions)[:n], (command, *self.commands)[:n])


class Observation(Protocol):
    history: int  # how many past commands build() reads

    def space(self) -> spaces.Box: ...

    def build(self, pool: Pool, history: History) -> np.ndarray:
        """The policy's input. It is given the pool and the commands only,
        never the capture, so it cannot reach the sites (rl-rewrite-plan.md,
        R1)."""


class Action(Protocol):
    history: int

    def command(self, action: np.ndarray, history: History) -> tuple[float, Command]:
        """The policy's action to (a as clipped, the command sent)."""


class Reward(Protocol):
    factors: tuple[str, ...]  # the names score() returns, in order, for the record

    def score(self, inputs: StepInputs) -> tuple[float, dict[str, float]]: ...


class End(Protocol):
    def check(self, inputs: StepInputs) -> str | None:
        """The reason the episode ends here, or None."""


@dataclass(frozen=True)
class Part:
    """A loaded plugin with the name and table it was loaded from, which the
    run's manifest records."""

    role: str
    name: str
    table: dict = field(hash=False)
    impl: Any = field(hash=False)


def available(role: str) -> list[str]:
    """The plugin files of `role`: every module on its package's path."""
    package = importlib.import_module(FOLDERS[role])
    return sorted(m.name for m in pkgutil.iter_modules(package.__path__))


def load(role: str, plugin: str, parent: dict, where: str, facts: RunFacts) -> Part:
    """The plugin `plugin` of `role`, built from its table `parent[plugin]`
    (empty when the gym file has none), or, for a plugin naming `TABLES`,
    from those tables of `parent`, keyed by name."""
    names = available(role)
    if plugin not in names:
        raise ValueError(f"{where}: no {role} plugin {plugin!r}; {FOLDERS[role]}/ holds {names}")
    module = importlib.import_module(f"{FOLDERS[role]}.{plugin}")
    shared = getattr(module.PLUGIN, "TABLES", None)
    if shared is not None:
        own = {name: parent.get(name, {}) for name in shared}
        for name, table in own.items():
            if not isinstance(table, dict):
                raise ValueError(f"{where}.{name}: a table, got {table!r}")
        return Part(role, plugin, {k: dict(v) for k, v in own.items()}, module.PLUGIN.from_table(own, facts, f"{where}."))
    own = parent.get(plugin, {})
    if not isinstance(own, dict):
        raise ValueError(f"{where}.{plugin}: a table, got {own!r}")
    return Part(role, plugin, dict(own), module.PLUGIN.from_table(own, facts, f"{where}.{plugin}."))
