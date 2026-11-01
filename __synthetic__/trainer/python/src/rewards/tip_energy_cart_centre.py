"""The tip, energy, cart and reach reward: __synthetic__/trainer/python/reward-plan.md's
calculation, the source of truth for every line here (sections 1 to 4).

  d        = |End - Dynamic Target|
  r_tip    = exp(-d / w)
  w0       = sqrt(m g l_c / I)
  spare    = 1/2 (speed / w0)^2 - (1 - (com.z - hinge.z) / l_c)
  r_energy = exp(-0.5 (K spare / spare_max)^2)
  c_d      = max(0, |Cart - Cart Target| - c_z)
  r_cart   = cart_floor + (1 - cart_floor) exp(-c_d / c_w)
  d_r      = |End - Target|
  r_reach  = reach_floor + (1 - reach_floor) exp(-d_r / reach_width)
  r        = r_tip * r_energy * r_cart * r_reach,  in (0, 1]

Every input is MuJoCo's (synthetic-loop-dia.md, The reward): End, Dynamic
Target, Target, Cart and Cart Target are capture t+1's sites; speed, com
and hinge its pole state (the pole joint's qvel, subtree_com and xanchor);
m, l_c, I and g the pole's body in the run's values. K = sqrt(-2 ln 0.1) =
2.146, so the bell is 0.1 at spare = +-spare_max (dm_control's tolerance,
__synthetic__/trainer/python/reward-plan.md 2.2.2). Dynamic Target rides the cart, so r_tip pays
a pole straight up the same anywhere on the rail; Target is fixed above the
middle. Straight up pays more than any lean while reach_width > w (1 -
reach_floor) (section 4).

The values sit one table per factor (user, 2026-10-29): [reward.tip]
width (w); [reward.energy] spare_max; [reward.cart] zone (c_z), width (c_w)
and floor; [reward.reach] width and floor.
"""

import math
from dataclasses import dataclass

import checks
from contracts import StepInputs
from plugins import RunFacts

# sqrt(-2 ln 0.1): dm_control's tolerance scale at value_at_margin 0.1
# (__synthetic__/trainer/python/reward-plan.md, 2.2.2).
K = math.sqrt(-2.0 * math.log(0.1))


@dataclass(frozen=True)
class TipEnergyCartCentre:
    width: float  # w, metres: r_tip is 1/e at this distance from Dynamic Target
    spare_max: float  # r_energy is 0.1 when spare is this far from 0
    omega0: float  # rad/s: the pole's natural frequency, from MuJoCo's body values
    com_distance: float  # l_c, metres: MuJoCo's centre-of-mass distance from the hinge axis
    cart_zone: float  # c_z, metres: the free zone's half-width, r_cart 1 inside it
    cart_width: float  # c_w, metres past the zone: the part of r_cart above the floor is 1/e there
    cart_floor: float  # r_cart's lower limit, approached far out, never reached
    reach_width: float  # metres from Target: the part of r_reach above the floor is 1/e there
    reach_floor: float  # r_reach's lower limit, approached far out, never reached

    TABLES = ("tip", "energy", "cart", "reach")
    factors = ("d", "tip", "spare", "energy", "c_d", "cart", "d_r", "reach")

    @staticmethod
    def from_table(tables: dict, facts: RunFacts, where: str) -> "TipEnergyCartCentre":
        tip, energy, cart, reach = (tables[name] for name in TipEnergyCartCentre.TABLES)
        checks.only(tip, {"width"}, f"{where}tip")
        checks.only(energy, {"spare_max"}, f"{where}energy")
        checks.only(cart, {"zone", "width", "floor"}, f"{where}cart")
        checks.only(reach, {"width", "floor"}, f"{where}reach")
        return TipEnergyCartCentre(
            width=checks.number(tip, "width", f"{where}tip.", 0.0, None, low_open=True),
            spare_max=checks.number(energy, "spare_max", f"{where}energy.", 0.0, None, low_open=True),
            omega0=math.sqrt(facts.pole_mass * facts.gravity * facts.pole_com_distance / facts.pole_inertia),
            com_distance=facts.pole_com_distance,
            cart_zone=checks.number(cart, "zone", f"{where}cart.", 0.0, None),
            cart_width=checks.number(cart, "width", f"{where}cart.", 0.0, None, low_open=True),
            cart_floor=checks.number(cart, "floor", f"{where}cart.", 0.0, 1.0),
            reach_width=checks.number(reach, "width", f"{where}reach.", 0.0, None, low_open=True),
            reach_floor=checks.number(reach, "floor", f"{where}reach.", 0.0, 1.0),
        )

    def score(self, inputs: StepInputs) -> tuple[float, dict[str, float]]:
        sites, pole = inputs.current, inputs.current_pole
        d = math.dist(sites.end, sites.dynamic_target)
        tip = math.exp(-d / self.width)
        moving = 0.5 * (pole.speed / self.omega0) ** 2
        height = 1 - (pole.com[2] - pole.hinge[2]) / self.com_distance
        spare = moving - height
        energy = math.exp(-0.5 * (K * spare / self.spare_max) ** 2)
        c_d = max(0.0, math.dist(sites.cart, sites.cart_target) - self.cart_zone)
        cart = self.cart_floor + (1 - self.cart_floor) * math.exp(-c_d / self.cart_width)
        d_r = math.dist(sites.end, sites.target)
        reach = self.reach_floor + (1 - self.reach_floor) * math.exp(-d_r / self.reach_width)
        factors = {"d": d, "tip": tip, "spare": spare, "energy": energy, "c_d": c_d, "cart": cart, "d_r": d_r,
                   "reach": reach}
        return tip * energy * cart * reach, factors


PLUGIN = TipEnergyCartCentre
