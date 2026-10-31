"""The contract types: what crosses the trainer link, checked once where it
enters, so everything inside the trainer can trust it (rl-rewrite-plan.md,
3.2).

- `Pool`: the policy's whole view of the rig, the Orin's last six marker
  frames (synthetic-loop-dia.md, Contracts, the pool).
- `Sites`: MuJoCo's true positions at the capture, for the reward and the
  episode end alone, never the policy (Contracts, the sites).
- `PoleState`: MuJoCo's true pole speed, angle, centre of mass and hinge
  point at the capture, for the reward and the record alone (Contracts,
  the pole).
- `Capture`: one instance's line entry (17): its id, MuJoCo time, pool,
  sites and pole state.
- `Command`: the i32 pulse rate the trainer answers with (18).
- `RunValues`: the orchestrator's answer to the trainer's request, once per
  run (Contracts, the run's values).
- `StepInputs`: everything the reward and the end conditions may read for
  one scored step, and nothing else. A plugin that needs another input adds
  it here first, so the step record and the replay buffer carry it too.
"""

import math
from dataclasses import dataclass

import numpy as np

# ---- the pool -------------------------------------------------------------

ENTRIES = 6
MARKERS = 4
ROW = 1 + MARKERS * 2  # an entry: its age, then (x, y) per marker
FLOATS = ENTRIES * ROW  # 54

# Slot order = the repo's class ids (AGENTS.md, Verified contracts).
LEFT, RIGHT, PIVOT, END = range(MARKERS)

NOT_DETECTED = (-1.0, -1.0)


def finite(value: object) -> bool:
    # bool is an int in Python; a value of `true` is a typo, not 1.
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


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
            raise ValueError(f"pool: holds {ENTRIES} entries, got {len(self.entries)}")
        if self.entries[0].age_ms != 0:
            raise ValueError(f"pool: the newest entry's age is 0, got {self.entries[0].age_ms}")
        ages = [entry.age_ms for entry in self.entries]
        if any(older >= newer for newer, older in zip(ages, ages[1:])):
            raise ValueError(f"pool: ages run newest first and fall strictly, got {ages}")
        for entry in self.entries:
            if len(entry.markers) != MARKERS:
                raise ValueError(f"pool: an entry holds {MARKERS} markers, got {len(entry.markers)}")
            for x, y in entry.markers:
                # -1 lies outside [0, 1], so "not detected" is never a position.
                if (x, y) != NOT_DETECTED and not (0 <= x <= 1 and 0 <= y <= 1):
                    raise ValueError(f"pool: a marker is in [0, 1] or (-1, -1), got ({x}, {y})")

    def flat(self) -> list[float]:
        """Per entry, newest first: the age, then x0, y0, ... x3, y3."""
        return [v for e in self.entries for v in (e.age_ms, *(c for m in e.markers for c in m))]

    @staticmethod
    def from_rows(rows: object) -> "Pool":
        """The pool as the link carries it (17): 6 rows, newest first, each
        [age_ms, x0, y0, ..., x3, y3]."""
        if (
            not isinstance(rows, list)
            or len(rows) != ENTRIES
            or not all(isinstance(row, list) and len(row) == ROW and all(finite(v) for v in row) for row in rows)
        ):
            raise ValueError(f"pool: {ENTRIES} rows of {ROW} finite numbers, got {rows!r}")
        return Pool.from_flat([float(v) for row in rows for v in row])

    @staticmethod
    def from_flat(values) -> "Pool":
        values = [float(v) for v in values]
        if len(values) != FLOATS:
            raise ValueError(f"pool: {FLOATS} floats flat, got {len(values)}")
        rows = [values[i : i + ROW] for i in range(0, FLOATS, ROW)]
        return Pool(tuple(Entry(r[0], tuple((r[1 + 2 * m], r[2 + 2 * m]) for m in range(MARKERS))) for r in rows))


# ---- the sites ------------------------------------------------------------

SITE_ROLES = ("left", "right", "pivot", "end", "target", "dynamic_target", "cart", "cart_target", "left_rail",
              "left_cart", "right_rail", "right_cart")
SITE_FLOATS = len(SITE_ROLES) * 3  # 36

Point = tuple[float, float, float]


@dataclass(frozen=True)
class Sites:
    """Twelve world points, MuJoCo's frame in metres, z up. Left, Right, Pivot
    and End are the markers' centres; Target is where End sits with the cart
    centred and the pole upright; Dynamic Target rides the cart where End
    sits with the pole upright; Cart rides the cart at the slider joint and
    Cart Target is where Cart sits with the cart centred; each rail site is where the cart would
    strike that base and each cart site the face that would strike it."""

    left: Point
    right: Point
    pivot: Point
    end: Point
    target: Point
    dynamic_target: Point
    cart: Point
    cart_target: Point
    left_rail: Point
    left_cart: Point
    right_rail: Point
    right_cart: Point

    def __post_init__(self) -> None:
        for role in SITE_ROLES:
            point = getattr(self, role)
            if not isinstance(point, tuple) or len(point) != 3 or not all(finite(v) for v in point):
                raise ValueError(f"sites: {role} is (x, y, z), finite metres, got {point!r}")

    def flat(self) -> list[float]:
        """Per role, in SITE_ROLES order: x, y, z."""
        return [v for role in SITE_ROLES for v in getattr(self, role)]

    @staticmethod
    def from_flat(values) -> "Sites":
        values = [float(v) for v in values]
        if len(values) != SITE_FLOATS:
            raise ValueError(f"sites: {SITE_FLOATS} floats flat, got {len(values)}")
        return Sites(**{role: tuple(values[3 * i : 3 * i + 3]) for i, role in enumerate(SITE_ROLES)})

    @staticmethod
    def from_message(value: object) -> "Sites":
        """A pool entry's "sites" (17): exactly the twelve roles, each [x, y, z]."""
        if not isinstance(value, dict) or set(value) != set(SITE_ROLES):
            raise ValueError(f"sites: exactly {{{', '.join(SITE_ROLES)}}}, got {value!r}")
        points = {}
        for role in SITE_ROLES:
            point = value[role]
            if not isinstance(point, list) or len(point) != 3 or not all(finite(v) for v in point):
                raise ValueError(f"sites: {role} is [x, y, z], finite metres, got {point!r}")
            points[role] = (float(point[0]), float(point[1]), float(point[2]))
        return Sites(**points)


# ---- the pole -------------------------------------------------------------

POLE_KEYS = ("speed", "angle", "com", "hinge")
POLE_FLOATS = 8  # speed, angle, then the centre's x, y, z, then the hinge's


def point(value: object, what: str) -> Point:
    """A message's [x, y, z] of finite metres."""
    if not isinstance(value, list) or len(value) != 3 or not all(finite(v) for v in value):
        raise ValueError(f"pole: {what} is [x, y, z], finite metres, got {value!r}")
    return (float(value[0]), float(value[1]), float(value[2]))


@dataclass(frozen=True)
class PoleState:
    """Everything that swings on the pole joint, at the capture, straight
    from MuJoCo (synthetic-loop-dia.md, Contracts, the pole)."""

    speed: float  # rad/s: the pole joint's qvel, signed about the joint's axis
    angle: float  # rad: the pole joint's qpos, 0 hanging, pi upright, counting on past a full turn
    com: Point  # the centre of mass, MuJoCo's subtree_com: world, metres, z up
    hinge: Point  # the hinge point, MuJoCo's xanchor of the pole joint, same frame

    def __post_init__(self) -> None:
        for name, unit in (("speed", "rad/s"), ("angle", "rad")):
            if not finite(getattr(self, name)):
                raise ValueError(f"pole: {name} is finite {unit}, got {getattr(self, name)!r}")
        for name in ("com", "hinge"):
            value = getattr(self, name)
            if not isinstance(value, tuple) or len(value) != 3 or not all(finite(v) for v in value):
                raise ValueError(f"pole: {name} is (x, y, z), finite metres, got {value!r}")

    def flat(self) -> list[float]:
        """speed, angle, then com's x, y, z, then hinge's x, y, z."""
        return [self.speed, self.angle, *self.com, *self.hinge]

    @staticmethod
    def from_flat(values) -> "PoleState":
        values = [float(v) for v in values]
        if len(values) != POLE_FLOATS:
            raise ValueError(f"pole: {POLE_FLOATS} floats flat, got {len(values)}")
        return PoleState(values[0], values[1], tuple(values[2:5]), tuple(values[5:8]))

    @staticmethod
    def from_message(value: object) -> "PoleState":
        """A pool entry's "pole" (17): exactly speed, angle, com and hinge."""
        if not isinstance(value, dict) or set(value) != set(POLE_KEYS):
            raise ValueError(f"pole: exactly {{{', '.join(POLE_KEYS)}}}, got {value!r}")
        for name, unit in (("speed", "rad/s"), ("angle", "rad")):
            if not finite(value[name]):
                raise ValueError(f"pole: {name} is finite {unit}, got {value[name]!r}")
        return PoleState(float(value["speed"]), float(value["angle"]), point(value["com"], "com"),
                         point(value["hinge"], "hinge"))


# ---- the capture ----------------------------------------------------------


@dataclass(frozen=True)
class Capture:
    """One instance's entry in a pools line (17): the pool, for the policy,
    and the same capture's MuJoCo time, sites and pole state, for the
    reward, the end and the record alone."""

    id: int  # the observation's id, counted across the run's episodes
    time: float  # s: MuJoCo's data.time at the capture, from 0 in each episode
    pool: Pool
    sites: Sites
    pole: PoleState

    def __post_init__(self) -> None:
        if not finite(self.time) or self.time < 0:
            raise ValueError(f"capture: time is seconds >= 0, got {self.time!r}")


# ---- the command ----------------------------------------------------------

# The link's command is an i32 (synthetic-loop-dia.md, Contracts, the
# command): a datatype limit. Its bound, +- the top pulse rate, is the run's
# (RunValues), checked by the environment before each answer.
I32_MIN, I32_MAX = -(2**31), 2**31 - 1


@dataclass(frozen=True)
class Command:
    """The answer's command (18): pulses per second, sign = direction."""

    rate: int

    def __post_init__(self) -> None:
        if isinstance(self.rate, bool) or not isinstance(self.rate, int):
            raise ValueError(f"command: a whole number of pulses/s, got {self.rate!r}")
        if not I32_MIN <= self.rate <= I32_MAX:
            raise ValueError(f"command: an i32, {I32_MIN} to {I32_MAX} pulses/s, got {self.rate}")


# ---- the run's values -----------------------------------------------------

RUN_KEYS = ("cart_top_speed", "top_pulse_rate", "timestep", "pole_mass", "pole_com_distance", "pole_inertia",
            "gravity")
# The pole's body: each a number > 0 (synthetic-loop-dia.md, Contracts, the
# run's values).
POLE_BODY_KEYS = RUN_KEYS[3:]


@dataclass(frozen=True)
class RunValues:
    """The orchestrator's answer to the trainer's request, from the model
    MuJoCo loaded and [drive] (synthetic-loop-dia.md, Contracts, the run's
    values). The model cannot change during the run, so one answer serves
    it."""

    cart_top_speed: float  # m/s: the MJCF actuator's ctrlrange
    top_pulse_rate: int  # pulses/s: cart top speed / metres per pulse, rounded down
    timestep: float  # s: MuJoCo's, the grid captures sit on
    pole_mass: float  # kg: everything that swings on the pole joint
    pole_com_distance: float  # m: its centre of mass's distance from the joint's axis
    pole_inertia: float  # kg m^2: its moment of inertia about that axis
    gravity: float  # m/s^2: the size of the model's gravity, straight down

    def __post_init__(self) -> None:
        for key in POLE_BODY_KEYS:
            value = getattr(self, key)
            if not finite(value) or value <= 0:
                raise ValueError(f"run: {key} is a number > 0, got {value!r}")
        if not finite(self.cart_top_speed) or self.cart_top_speed <= 0:
            raise ValueError(f"run: cart_top_speed is m/s > 0, got {self.cart_top_speed!r}")
        if isinstance(self.top_pulse_rate, bool) or not isinstance(self.top_pulse_rate, int) \
                or not 1 <= self.top_pulse_rate <= I32_MAX:
            raise ValueError(f"run: top_pulse_rate is a whole number of pulses/s, 1 to {I32_MAX}, "
                             f"got {self.top_pulse_rate!r}")
        if not finite(self.timestep) or self.timestep <= 0:
            raise ValueError(f"run: timestep is seconds > 0, got {self.timestep!r}")

    def message(self) -> dict:
        """As the answer carries it, and the run's manifest records it."""
        return {key: getattr(self, key) for key in RUN_KEYS}

    @staticmethod
    def from_message(value: object) -> "RunValues":
        """The answer's "run": exactly the seven values."""
        if not isinstance(value, dict) or set(value) != set(RUN_KEYS):
            raise ValueError(f"run: exactly {{{', '.join(RUN_KEYS)}}}, got {value!r}")
        # The rate stays as sent: a float rate is refused, not rounded.
        return RunValues(**{key: value[key] if key == "top_pulse_rate" or not finite(value[key])
                            else float(value[key]) for key in RUN_KEYS})


# ---- the step's inputs ----------------------------------------------------


@dataclass(frozen=True)
class StepInputs:
    """One scored step t: command c_t, scored with capture t+1
    (synthetic-loop-dia.md, The reward)."""

    previous: Sites  # capture t's sites
    current: Sites  # capture t+1's sites
    command: int  # c_t, pulses/s
    previous_command: int  # c_(t-1), pulses/s; 0 on an episode's first scored step
    step: int  # t within the episode, 0 for the first scored step
    previous_pole: PoleState  # capture t's pole state
    current_pole: PoleState  # capture t+1's pole state

    # The flat layout the step record and the replay buffer store: the two
    # captures' sites, the three numbers, then the two pole states.
    SIZE = 2 * SITE_FLOATS + 3 + 2 * POLE_FLOATS  # 91

    def __post_init__(self) -> None:
        Command(self.command)
        Command(self.previous_command)
        if isinstance(self.step, bool) or not isinstance(self.step, int) or self.step < 0:
            raise ValueError(f"step inputs: step is a whole number >= 0, got {self.step!r}")

    def to_array(self) -> np.ndarray:
        return np.array(
            [*self.previous.flat(), *self.current.flat(), self.command, self.previous_command, self.step,
             *self.previous_pole.flat(), *self.current_pole.flat()],
            dtype=np.float64,
        )

    @staticmethod
    def from_array(values) -> "StepInputs":
        values = np.asarray(values, dtype=np.float64).reshape(-1)
        if len(values) != StepInputs.SIZE:
            raise ValueError(f"step inputs: {StepInputs.SIZE} floats flat, got {len(values)}")
        tail = values[2 * SITE_FLOATS : 2 * SITE_FLOATS + 3]
        poles = values[2 * SITE_FLOATS + 3 :]
        return StepInputs(
            previous=Sites.from_flat(values[:SITE_FLOATS]),
            current=Sites.from_flat(values[SITE_FLOATS : 2 * SITE_FLOATS]),
            command=int(tail[0]),
            previous_command=int(tail[1]),
            step=int(tail[2]),
            previous_pole=PoleState.from_flat(poles[:POLE_FLOATS]),
            current_pole=PoleState.from_flat(poles[POLE_FLOATS:]),
        )
