"""DeepMind's cart-pole reward, on this rig: __synthetic__/trainer/python/dm_control_reward.md's
calculation (sections 0 to 4), read from dm_control's cartpole.py.

  cos_theta      = -cos(angle)
  upright        = (cos_theta + 1) / 2
  u              = command / top pulse rate
  small_control  = control_floor + (1 - control_floor) max(0, 1 - (|u| / control_margin)^2)
  omega          = |speed|
  small_velocity = velocity_floor + (1 - velocity_floor) bell(omega, velocity_margin, velocity_at_margin)
  x              = |Cart - Cart Target|
  centered       = centered_floor + (1 - centered_floor) bell(x, centered_margin, centered_at_margin)
  r              = upright * small_control * small_velocity * centered,  in [0, 1]

  bell(d, margin, at_margin) = at_margin ** ((d / margin)^2)

bell is dm_control's Gaussian tolerance() with its band at 0: 1 at d = 0, at_margin at
d = margin, written so that it can be checked by hand. It is the same number as the
document's exp(-1/2 (d / margin x scale)^2) with scale = sqrt(-2 ln at_margin), since
that exponent is (d / margin)^2 x ln at_margin. At 5 rad/s, margin 5, at_margin 0.1:
0.1 ** 1 = 0.1, the document's table.

Every input but the command is MuJoCo's (__documentation__/synthetic-loop-dia.md, The reward), from
capture t+1: angle and speed its pole state (the pole joint's qpos and qvel), Cart and
Cart Target its sites. angle is 0 hanging and pi upright, while DeepMind's theta is 0
upright, hence the minus sign in cos_theta.

Where this rig differs from DeepMind's model:
- u is the policy's command, c_t, as a fraction of the top pulse rate. It stands where
  DeepMind's motor command stands, but DeepMind's is a force and this one a cart speed,
  so small_control charges commanding a fast cart, not a hard push.
- Hanging exactly straight down scores exactly 0, as in DeepMind's, however still the
  pole is, so r is in [0, 1], with 0 included.

The values sit one table per factor: [reward.small_control] margin and floor;
[reward.small_velocity] and [reward.centered] margin, at_margin and floor. upright has
none. DeepMind's fixed numbers are its starting values (gym.toml).
"""

import math
from dataclasses import dataclass

import checks
from contracts import StepInputs
from plugins import RunFacts


def bell(distance: float, margin: float, at_margin: float) -> float:
    """1 at no distance, at_margin at one margin, falling towards 0 beyond it."""
    return at_margin ** ((distance / margin) ** 2)


@dataclass(frozen=True)
class DmControlCartpole:
    top_pulse_rate: int  # pulses/s: the command that makes u = 1, the run's value
    control_margin: float  # |u| at which the part of small_control above the floor reaches 0
    control_floor: float  # small_control's lowest value, reached at that |u|
    velocity_margin: float  # rad/s: the speed at which the bell reaches velocity_at_margin
    velocity_at_margin: float  # the bell's value at that speed
    velocity_floor: float  # small_velocity's lower limit, approached at high speed, never reached
    centered_margin: float  # metres from the middle: where the bell reaches centered_at_margin
    centered_at_margin: float  # the bell's value at that distance
    centered_floor: float  # centered's lower limit, approached far out, never reached

    TABLES = ("small_control", "small_velocity", "centered")
    factors = ("cos_theta", "upright", "u", "small_control", "omega", "small_velocity", "x", "centered")

    @staticmethod
    def from_table(tables: dict, facts: RunFacts, where: str) -> "DmControlCartpole":
        control, velocity, centered = (tables[name] for name in DmControlCartpole.TABLES)
        checks.only(control, {"margin", "floor"}, f"{where}small_control")
        checks.only(velocity, {"margin", "at_margin", "floor"}, f"{where}small_velocity")
        checks.only(centered, {"margin", "at_margin", "floor"}, f"{where}centered")
        return DmControlCartpole(
            top_pulse_rate=facts.top_pulse_rate,
            control_margin=checks.number(control, "margin", f"{where}small_control.", 0.0, None, low_open=True),
            control_floor=checks.number(control, "floor", f"{where}small_control.", 0.0, 1.0),
            velocity_margin=checks.number(velocity, "margin", f"{where}small_velocity.", 0.0, None, low_open=True),
            velocity_at_margin=checks.number(velocity, "at_margin", f"{where}small_velocity.", 0.0, 1.0, low_open=True),
            velocity_floor=checks.number(velocity, "floor", f"{where}small_velocity.", 0.0, 1.0),
            centered_margin=checks.number(centered, "margin", f"{where}centered.", 0.0, None, low_open=True),
            centered_at_margin=checks.number(centered, "at_margin", f"{where}centered.", 0.0, 1.0, low_open=True),
            centered_floor=checks.number(centered, "floor", f"{where}centered.", 0.0, 1.0),
        )

    def score(self, inputs: StepInputs) -> tuple[float, dict[str, float]]:
        sites, pole = inputs.current, inputs.current_pole
        cos_theta = -math.cos(pole.angle)
        upright = (cos_theta + 1) / 2
        u = inputs.command / self.top_pulse_rate
        small_control = self.control_floor + (1 - self.control_floor) * max(0.0, 1 - (u / self.control_margin) ** 2)
        omega = abs(pole.speed)
        small_velocity = self.velocity_floor + (1 - self.velocity_floor) * bell(
            omega, self.velocity_margin, self.velocity_at_margin)
        x = math.dist(sites.cart, sites.cart_target)
        centered = self.centered_floor + (1 - self.centered_floor) * bell(
            x, self.centered_margin, self.centered_at_margin)
        factors = {"cos_theta": cos_theta, "upright": upright, "u": u, "small_control": small_control,
                   "omega": omega, "small_velocity": small_velocity, "x": x, "centered": centered}
        return upright * small_control * small_velocity * centered, factors


PLUGIN = DmControlCartpole
