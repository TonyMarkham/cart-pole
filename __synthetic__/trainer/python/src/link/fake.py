"""The fake link: a scripted rig in Python, for the tests.

It speaks the link's interface exactly as the real one does (ids, pools,
sites, pole states, the run's values, rounds, resets in place, stop), so the environment, the buffer and
the training run can be tested with no orchestrator, Unity player or Orin.
It is a test fixture, not a model of the rig: the cart follows the command
at once (MuJoCo's velocity servo, idealised), the pole is a damped
pendulum on the cart, and the markers are the sites projected linearly
into the frame, always detected. The geometry copies the sites
(synthetic-loop-dia.md, Contracts, the sites, as MuJoCo gives them): End 0.450 m from
the hinge, the rail centre 10 mm off the world origin, Target at
(-0.010, -0.132, 0.532), Dynamic Target 0.450 m above the hinge, Cart at the
slider joint (y 0, z 0.050) and Cart Target where it sits centred, and the
strike gaps 0.440 m left and 0.445 m right with the cart centred.
"""

import math
import random
from fractions import Fraction

from contracts import ENTRIES, MARKERS, NOT_DETECTED, Capture, Pool, Entry, PoleState, RunValues, Sites
from link import RunFailed
from link.answer import CONTINUE, RESET, STOP, Answer

RAIL_X = -0.010  # the rail centre in world x
HINGE_Y, HINGE_Z = -0.132, 0.532 - 0.450  # Target less the pole: the hinge's height
CART_Y, CART_Z = 0.0, 0.050  # the slider joint, where Cart and Cart Target sit
POLE = 0.450  # hinge to End
HALF_CART = 0.05  # the cart's half width (the fixture's own)
LEFT_TRAVEL, RIGHT_TRAVEL = -0.440, 0.445  # where the cart's faces meet the bases

# The pendulum's own constants: an effective length and a damping, rad/s per rad/s.
SWING_LENGTH, DAMPING, G = 0.3, 0.05, 9.81
# The pole's body as the fixture answers it (the run's values): a mass and a
# centre-of-mass distance of its own, and the inertia that makes them swing
# like the pendulum above, I = m l_c L (a pendulum's effective length is
# I / (m l_c)).
POLE_MASS, POLE_COM = 0.4, 0.15
POLE_INERTIA = POLE_MASS * POLE_COM * SWING_LENGTH


class Rig:
    """One instance's rig state: cart position (m, from the rail centre),
    cart speed, pole angle (0 hanging, pi upright) and its rate."""

    def __init__(self) -> None:
        self.cart = self.speed = self.pole = self.rate = 0.0

    def step(self, speed: float, dt_s: float) -> None:
        """One timestep with the cart at `speed`, m/s."""
        accel = (speed - self.speed) / dt_s
        self.speed = speed
        alpha = -(G * math.sin(self.pole) + accel * math.cos(self.pole)) / SWING_LENGTH - DAMPING * self.rate
        self.rate += alpha * dt_s
        self.pole += self.rate * dt_s
        self.cart += self.speed * dt_s
        if not LEFT_TRAVEL <= self.cart <= RIGHT_TRAVEL:  # the bases stop it
            self.cart = min(max(self.cart, LEFT_TRAVEL), RIGHT_TRAVEL)
            self.speed = 0.0

    def sites(self) -> Sites:
        hinge = (RAIL_X + self.cart, HINGE_Y, HINGE_Z)
        end = (hinge[0] + POLE * math.sin(self.pole), HINGE_Y, HINGE_Z - POLE * math.cos(self.pole))
        return Sites(
            left=(RAIL_X - 0.47, HINGE_Y, 0.0), right=(RAIL_X + 0.47, HINGE_Y, 0.0), pivot=hinge, end=end,
            target=(RAIL_X, HINGE_Y, HINGE_Z + POLE), dynamic_target=(hinge[0], HINGE_Y, HINGE_Z + POLE),
            cart=(hinge[0], CART_Y, CART_Z), cart_target=(RAIL_X, CART_Y, CART_Z),
            left_rail=(RAIL_X + LEFT_TRAVEL - HALF_CART, HINGE_Y, HINGE_Z), left_cart=(hinge[0] - HALF_CART, HINGE_Y, HINGE_Z),
            right_rail=(RAIL_X + RIGHT_TRAVEL + HALF_CART, HINGE_Y, HINGE_Z), right_cart=(hinge[0] + HALF_CART, HINGE_Y, HINGE_Z),
        )

    def pole_state(self) -> PoleState:
        """The pole's speed and angle, its centre of mass, on the line from
        the hinge to End, and the hinge point."""
        hinge = (RAIL_X + self.cart, HINGE_Y, HINGE_Z)
        com = (hinge[0] + POLE_COM * math.sin(self.pole), HINGE_Y, HINGE_Z - POLE_COM * math.cos(self.pole))
        return PoleState(self.rate, self.pole, com, hinge)


def project(sites: Sites) -> tuple[tuple[float, float], ...]:
    """Pivot and End as the Orin sends them: (x, z), in metres, a perfect matrix's."""
    return tuple((p[0], p[2]) for p in (sites.pivot, sites.end))


def top_pulse_rate(cart_top_speed: float, metres_per_pulse: float) -> int:
    """Cart top speed / metres per pulse, rounded down, as the orchestrator
    answers it; taken on the values as written (0.9 / 15e-6 is 60,000, not a
    float division's 59,999.99...)."""
    return math.floor(Fraction(repr(cart_top_speed)) / Fraction(repr(metres_per_pulse)))


class Instance:
    def __init__(self, seed: int, metres_per_pulse: float, timestep_ms: float, capture_gaps: tuple[float, float],
                 stop_gap: float) -> None:
        self.metres_per_pulse = metres_per_pulse
        self.timestep_ms = timestep_ms
        self.capture_gaps = capture_gaps  # ms: the shortest and longest gap between captures, before rounding
        self.stop_gap = stop_gap  # m: a limit switch closes within this of its base
        self.next_id = 0
        self.reset(seed)

    def gap_ms(self) -> float:
        return self.random.uniform(*self.capture_gaps)

    def reset(self, seed: int) -> None:
        """Reset in place (R2-R6): the start pose at t = 0, out of FAULT, and
        a pool pre-warmed afresh, its gaps unrounded picks from the seed."""
        self.random = random.Random(seed)
        self.rig = Rig()
        self.time_ms = 0.0
        self.fault = False
        t, self.frames = 0.0, []
        for _ in range(ENTRIES):
            t -= self.gap_ms()
            self.frames.append((t, (NOT_DETECTED,) * MARKERS))
        self.frames.reverse()  # oldest first

    def capture(self) -> Capture:
        sites = self.rig.sites()
        self.frames = self.frames[1:] + [(self.time_ms, project(sites))]
        pool = Pool(tuple(Entry(t - self.time_ms, markers) for t, markers in reversed(self.frames)))
        capture = Capture(self.next_id, self.time_ms / 1000, pool, sites, self.rig.pole_state(), self.fault)
        self.next_id += 1
        return capture

    def tripped(self) -> bool:
        """A limit switch is closed: a strike face within the stop gap of its
        base (synthetic-loop-dia.md, The rail's ends)."""
        s = self.rig.sites()
        return s.left_cart[0] - s.left_rail[0] <= self.stop_gap or s.right_rail[0] - s.right_cart[0] <= self.stop_gap

    def advance(self, command: int) -> None:
        """The cart runs on `command` to the next capture, a gap picked
        between the shortest and longest the bands allow, rounded to the
        nearest timestep. The fixture's own timing, not the loop's: the
        command acts at once, with nothing in flight. A tripped switch is a
        FAULT: the cart stops there, and the next capture says so."""
        if self.fault:
            raise RuntimeError("the fake rig is in FAULT: only a reset or a stop answers it")
        steps = max(1, math.floor(self.gap_ms() / self.timestep_ms + 0.5))
        for _ in range(steps):
            self.rig.step(command * self.metres_per_pulse, self.timestep_ms / 1000)
            self.time_ms += self.timestep_ms
            if self.tripped():
                self.fault = True
                break


class FakeLink:
    def __init__(self, instances: int, seed: int, cart_top_speed: float, metres_per_pulse: float, timestep: float,
                 capture_gaps: tuple[float, float] = (15.5, 49.5), stop_gap: float = 0.005,
                 fail_after: int | None = None) -> None:
        """The tests give the run's values: the cart top speed (m/s), metres
        per pulse and the timestep (s); ask() answers with them, the top
        pulse rate derived as the orchestrator derives it. `capture_gaps`,
        ms, defaults to today's bands' 15.5 to 49.5 ms, and `stop_gap`, m, to
        today's 5 mm."""
        self.instances = instances
        self.values = RunValues(cart_top_speed, top_pulse_rate(cart_top_speed, metres_per_pulse), timestep,
                                POLE_MASS, POLE_COM, POLE_INERTIA, G)
        # Instance k's first episode runs with seed + k, as --seed does.
        self.rigs = [Instance(seed + k, metres_per_pulse, timestep * 1000, capture_gaps, stop_gap)
                     for k in range(instances)]
        self.pending = {k: self.rigs[k].capture() for k in range(instances)}
        self.stopped = False
        self.fail_after = fail_after  # lines before the run fails, for the tests
        self.lines = 0

    def ask(self) -> RunValues:
        return self.values

    def next_captures(self) -> dict[int, Capture]:
        assert not self.stopped and self.pending
        self.lines += 1
        if self.fail_after is not None and self.lines > self.fail_after:
            raise RunFailed("the fake run failed, as scripted")
        captures, self.pending = self.pending, {}
        return captures

    def answer(self, answers: dict[int, Answer]) -> None:
        assert not self.stopped and not self.pending
        if any(a.next == STOP for a in answers.values()):
            self.stopped = True
            return
        for k, a in sorted(answers.items()):
            rig = self.rigs[k]
            if a.next == CONTINUE:
                rig.advance(a.command.rate)
            elif a.next == RESET:
                rig.reset(a.seed)
            self.pending[k] = rig.capture()

    def finish(self) -> int:
        assert self.stopped
        return 0

    def kill(self) -> None:
        pass
