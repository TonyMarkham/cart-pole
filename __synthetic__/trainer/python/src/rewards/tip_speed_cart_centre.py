"""The tip, target speed, cart, reach and speed reward: __synthetic__/trainer/python/reward-plan.md's
calculation, the source of truth for every line here (sections 1 to 5).

  d              = |End - Dynamic Target|
  r_tip          = exp(-d / w)
  d_x            = |End.x - Dynamic Target.x|
  in the zone    = d_x <= zone_dx and End.z > Pivot.z
  over_target    = max(0, |speed| - target_limit) in the zone, otherwise 0
  r_target_speed = target_floor + (1 - target_floor) exp(-over_target / target_width)
  c_d            = max(0, |Cart - Cart Target| - c_z)
  r_cart         = cart_floor + (1 - cart_floor) exp(-c_d / c_w)
  d_r            = |End - Target|
  r_reach        = reach_floor + (1 - reach_floor) exp(-d_r / reach_width)
  over_speed     = max(0, |speed| - speed_limit)
  r_speed        = speed_floor + (1 - speed_floor) exp(-over_speed / speed_width)
  r              = r_tip * r_target_speed * r_cart * r_reach * r_speed,  in (0, 1]

Every input is MuJoCo's (synthetic-loop-dia.md, The reward): End, Dynamic
Target, Target, Pivot, Cart and Cart Target are capture t+1's sites; speed its
pole state (the pole joint's qvel). Dynamic Target rides the cart, so r_tip
pays a pole straight up the same anywhere on the rail, and d_x is 0 for it
anywhere; a hanging pole's d_x is 0 too, which is why the zone also needs End
above Pivot (section 2.1.2.3). Target is fixed above the middle. Straight up
pays more than any lean while reach_width > w (1 - reach_floor) (section 4).

The values sit one table per factor (user, 2026-10-29): [reward.tip] width
(w); [reward.target_speed] zone_dx, limit, width and floor; [reward.cart] zone
(c_z), width (c_w) and floor; [reward.reach] width and floor; [reward.speed]
limit, width and floor.
"""

import math
from dataclasses import dataclass

import checks
from contracts import StepInputs
from plugins import RunFacts


@dataclass(frozen=True)
class TipSpeedCartCentre:
    width: float  # w, metres: r_tip is 1/e at this distance from Dynamic Target
    zone_dx: float  # metres along the rail from Dynamic Target: the zone's half-width
    target_limit: float  # rad/s: r_target_speed is 1 up to this speed in the zone
    target_width: float  # rad/s past the limit: the part of r_target_speed above the floor is 1/e there
    target_floor: float  # r_target_speed's lower limit, approached at high speed, never reached
    cart_zone: float  # c_z, metres: the free zone's half-width, r_cart 1 inside it
    cart_width: float  # c_w, metres past the zone: the part of r_cart above the floor is 1/e there
    cart_floor: float  # r_cart's lower limit, approached far out, never reached
    reach_width: float  # metres from Target: the part of r_reach above the floor is 1/e there
    reach_floor: float  # r_reach's lower limit, approached far out, never reached
    speed_limit: float  # rad/s: r_speed is 1 up to this speed, anywhere
    speed_width: float  # rad/s past the limit: the part of r_speed above the floor is 1/e there
    speed_floor: float  # r_speed's lower limit, approached at high speed, never reached

    TABLES = ("tip", "target_speed", "cart", "reach", "speed")
    factors = ("d", "tip", "d_x", "over_target", "target_speed", "c_d", "cart", "d_r", "reach", "over_speed", "speed")

    @staticmethod
    def from_table(tables: dict, facts: RunFacts, where: str) -> "TipSpeedCartCentre":
        tip, target, cart, reach, speed = (tables[name] for name in TipSpeedCartCentre.TABLES)
        checks.only(tip, {"width"}, f"{where}tip")
        checks.only(target, {"zone_dx", "limit", "width", "floor"}, f"{where}target_speed")
        checks.only(cart, {"zone", "width", "floor"}, f"{where}cart")
        checks.only(reach, {"width", "floor"}, f"{where}reach")
        checks.only(speed, {"limit", "width", "floor"}, f"{where}speed")
        return TipSpeedCartCentre(
            width=checks.number(tip, "width", f"{where}tip.", 0.0, None, low_open=True),
            zone_dx=checks.number(target, "zone_dx", f"{where}target_speed.", 0.0, None),
            target_limit=checks.number(target, "limit", f"{where}target_speed.", 0.0, None),
            target_width=checks.number(target, "width", f"{where}target_speed.", 0.0, None, low_open=True),
            target_floor=checks.number(target, "floor", f"{where}target_speed.", 0.0, 1.0),
            cart_zone=checks.number(cart, "zone", f"{where}cart.", 0.0, None),
            cart_width=checks.number(cart, "width", f"{where}cart.", 0.0, None, low_open=True),
            cart_floor=checks.number(cart, "floor", f"{where}cart.", 0.0, 1.0),
            reach_width=checks.number(reach, "width", f"{where}reach.", 0.0, None, low_open=True),
            reach_floor=checks.number(reach, "floor", f"{where}reach.", 0.0, 1.0),
            speed_limit=checks.number(speed, "limit", f"{where}speed.", 0.0, None),
            speed_width=checks.number(speed, "width", f"{where}speed.", 0.0, None, low_open=True),
            speed_floor=checks.number(speed, "floor", f"{where}speed.", 0.0, 1.0),
        )

    def score(self, inputs: StepInputs) -> tuple[float, dict[str, float]]:
        sites, turning = inputs.current, abs(inputs.current_pole.speed)
        d = math.dist(sites.end, sites.dynamic_target)
        tip = math.exp(-d / self.width)
        d_x = abs(sites.end[0] - sites.dynamic_target[0])
        in_zone = d_x <= self.zone_dx and sites.end[2] > sites.pivot[2]
        over_target = max(0.0, turning - self.target_limit) if in_zone else 0.0
        target_speed = self.target_floor + (1 - self.target_floor) * math.exp(-over_target / self.target_width)
        c_d = max(0.0, math.dist(sites.cart, sites.cart_target) - self.cart_zone)
        cart = self.cart_floor + (1 - self.cart_floor) * math.exp(-c_d / self.cart_width)
        d_r = math.dist(sites.end, sites.target)
        reach = self.reach_floor + (1 - self.reach_floor) * math.exp(-d_r / self.reach_width)
        over_speed = max(0.0, turning - self.speed_limit)
        speed = self.speed_floor + (1 - self.speed_floor) * math.exp(-over_speed / self.speed_width)
        factors = {"d": d, "tip": tip, "d_x": d_x, "over_target": over_target, "target_speed": target_speed,
                   "c_d": c_d, "cart": cart, "d_r": d_r, "reach": reach, "over_speed": over_speed, "speed": speed}
        return tip * target_speed * cart * reach * speed, factors


PLUGIN = TipSpeedCartCentre
