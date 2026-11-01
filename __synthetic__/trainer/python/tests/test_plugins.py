"""The plugins against hand-computed values (rl-rewrite-plan.md, build
steps 4 and 5): the reward against __synthetic__/trainer/python/reward-plan.md's tables, the
strike gap at the measured gaps, the observation's shape and contents, the
action's rounding and bound."""

import inspect
import math
import unittest

import support
from contracts import StepInputs
from endings.strike_gap import StrikeGap
from link.fake import Rig
from observations.pool_and_commands import PoolAndCommands
from actions.linear_rate import LinearRate
from plugins import History
from rewards.tip_energy_cart_centre import TipEnergyCartCentre
from rewards.tip_speed_cart_centre import TipSpeedCartCentre

FACTS = support.facts(15.0)
TOP = FACTS.top_pulse_rate  # 60,000 pulses/s at the tests' run values
TABLES = {"tip": {"width": 0.2}, "energy": {"spare_max": 2.0}, "cart": {"zone": 0.050, "width": 0.25, "floor": 0.667},
          "reach": {"width": 0.2, "floor": 0.667}}
REWARD = TipEnergyCartCentre.from_table(TABLES, FACTS, "reward.")
SPEED_TABLES = {"tip": {"width": 0.2}, "target_speed": {"zone_dx": 0.154, "limit": 0.5, "width": 1.0, "floor": 0.667},
                "cart": {"zone": 0.050, "width": 0.25, "floor": 0.667}, "reach": {"width": 0.2, "floor": 0.667},
                "speed": {"limit": 12.0, "width": 1.0, "floor": 0.250}}
SPEED_REWARD = TipSpeedCartCentre.from_table(SPEED_TABLES, FACTS, "reward.")


def rig(cart=0.0, pole=math.pi, speed=0.0):
    """The fixture rig: pole 0 hanging, pi upright, turning at `speed` rad/s."""
    r = Rig()
    r.cart, r.pole, r.rate = cart, pole, speed
    return r


def inputs(now, before=None, command=0, previous=0):
    before = before or now
    return StepInputs(before.sites(), now.sites(), command, previous, 0, before.pole_state(), now.pole_state())


class TipEnergyCartCentreReward(unittest.TestCase):
    def test_tip_table(self):
        """__synthetic__/trainer/python/reward-plan.md section 1, w = 0.2 m: the star rides the
        cart, so straight up is 1 wherever the cart is."""
        table = [
            (rig(), 0.0, 1.00),
            (rig(cart=0.2), 0.0, 1.00),
            (rig(cart=-0.440), 0.0, 1.00),
            (rig(pole=math.pi - math.radians(10)), 0.078, 0.68),
            (rig(cart=0.2, pole=math.pi - math.radians(10)), 0.078, 0.68),
            (rig(pole=math.pi / 2), 0.636, 0.042),
            (rig(pole=0.0), 0.900, 0.011),
        ]
        for now, d, tip in table:
            _, f = REWARD.score(inputs(now))
            self.assertAlmostEqual(f["d"], d, places=3)
            self.assertAlmostEqual(f["tip"], tip, places=2 if tip >= 0.1 else 3)

    def test_energy_table(self):
        """__synthetic__/trainer/python/reward-plan.md sections 2.1 and 2.2 at spare_max = 2: the
        bell is 0.1 ** (spare / spare_max) ** 2. omega0 comes from the run's
        values (the fixture's pole body here)."""
        omega0 = REWARD.omega0
        table = [
            (rig(), 0.0, 1.0),                                       # upright, still
            (rig(pole=0.0), -2.0, 0.1),                              # hanging, still
            (rig(pole=math.pi / 2), -1.0, 0.1 ** 0.25),              # sideways, still
            (rig(pole=0.0, speed=2 * omega0), 0.0, 1.0),             # passing the bottom with just enough
            (rig(speed=omega0), 0.5, 0.1 ** (1 / 16)),               # passing straight up at omega0
            (rig(speed=-omega0), 0.5, 0.1 ** (1 / 16)),              # either way round
        ]
        for now, spare, energy in table:
            _, f = REWARD.score(inputs(now))
            self.assertAlmostEqual(f["spare"], spare, places=12)
            self.assertAlmostEqual(f["energy"], energy, places=12)

    def test_omega0_from_the_run_values(self):
        """omega0 = sqrt(m g l_c / I), MuJoCo's body values (the run's)."""
        self.assertAlmostEqual(
            REWARD.omega0,
            math.sqrt(FACTS.pole_mass * FACTS.gravity * FACTS.pole_com_distance / FACTS.pole_inertia), places=15)

    def test_cart_table(self):
        """__synthetic__/trainer/python/reward-plan.md section 3, c_z 0.050 m, c_w 0.25 m, floor
        0.667, from |Cart - Cart Target|; left and right count the same."""
        table = [
            (0.0, 0.0, 1.0),
            (0.03, 0.0, 1.0),
            (-0.05, 0.0, 1.0),
            (0.1, 0.05, 0.940),
            (-0.2, 0.15, 0.850),
            (0.3, 0.25, 0.790),
            (-0.44, 0.39, 0.737),
        ]
        for cart, c_d, score in table:
            _, f = REWARD.score(inputs(rig(cart=cart)))
            self.assertAlmostEqual(f["c_d"], c_d, places=12)
            self.assertAlmostEqual(f["cart"], score, places=3)

    def test_reach_table(self):
        """__synthetic__/trainer/python/reward-plan.md section 4, width 0.2 m, floor 0.667, from
        |End - Target|, Target fixed above the middle."""
        table = [
            (rig(), 0.0, 1.0),
            (rig(cart=0.2), 0.2, 0.790),
            (rig(cart=-0.44), 0.44, 0.704),
            (rig(pole=0.0), 0.9, 0.671),
        ]
        for now, d_r, score in table:
            _, f = REWARD.score(inputs(now))
            self.assertAlmostEqual(f["d_r"], d_r, places=12)
            self.assertAlmostEqual(f["reach"], score, places=3)

    def test_never_reaches_the_floors(self):
        """Squeezed, not clamped: past the zone every step towards the middle
        pays, out to the ends (sections 3.2 and 4.2)."""
        for factor in ("cart", "reach"):
            scores = [REWARD.score(inputs(rig(cart=-x / 100)))[1][factor] for x in range(5, 45)]
            self.assertTrue(all(a > b for a, b in zip(scores, scores[1:])), factor)
            self.assertGreater(scores[-1], 0.667, factor)

    def test_straight_up_beats_any_lean(self):
        """__synthetic__/trainer/python/reward-plan.md section 4: with the cart off the middle,
        leaning the pole still towards the middle never pays more than
        straight up, at reach_width > w (1 - reach_floor)."""
        for cart in (0.05, 0.1, 0.2, 0.3, 0.44, -0.2, -0.44):
            towards = 1 if cart > 0 else -1  # pole pi + a puts End at -sin(a): towards -x for a > 0
            upright = REWARD.score(inputs(rig(cart=cart)))[0]
            for tenths in range(1, 401):
                lean = REWARD.score(inputs(rig(cart=cart, pole=math.pi + towards * math.radians(tenths / 10))))[0]
                self.assertLess(lean, upright, f"cart {cart} m, lean {tenths / 10} deg")

    def test_product_range_and_commands(self):
        """r = r_tip x r_energy x r_cart x r_reach in (0, 1], whatever the
        commands; 1 upright and still at centre."""
        now, before = rig(cart=0.2, pole=1.0, speed=3.0), rig(cart=0.2, pole=0.8, speed=2.0)
        r, f = REWARD.score(inputs(now, before))
        jumped, _ = REWARD.score(inputs(now, before, command=20000, previous=-20000))
        self.assertEqual(r, jumped)
        self.assertAlmostEqual(r, f["tip"] * f["energy"] * f["cart"] * f["reach"], places=15)
        self.assertTrue(0 < r <= 1)
        self.assertAlmostEqual(REWARD.score(inputs(rig()))[0], 1.0, places=12)
        self.assertEqual(REWARD.factors, ("d", "tip", "spare", "energy", "c_d", "cart", "d_r", "reach"))

    def test_table_checks(self):
        for factor, change, pattern in (("tip", {"width": 0.0}, r"reward\.tip\.width"),
                                        ("energy", {"spare_max": 0.0}, r"reward\.energy\.spare_max"),
                                        ("energy", {"spare_max": None}, r"reward\.energy.*spare_max"),
                                        ("cart", {"zone": -0.01}, r"reward\.cart\.zone"),
                                        ("cart", {"width": 0.0}, r"reward\.cart\.width"),
                                        ("cart", {"floor": 1.5}, r"reward\.cart\.floor"),
                                        ("reach", {"width": 0.0}, r"reward\.reach\.width"),
                                        ("reach", {"floor": None}, r"reward\.reach.*floor"),
                                        ("tip", {"speed": 5.0}, r"reward\.tip: unknown")):
            changed = {k: v for k, v in {**TABLES[factor], **change}.items() if v is not None}
            with self.assertRaisesRegex(ValueError, pattern):
                TipEnergyCartCentre.from_table({**TABLES, factor: changed}, FACTS, "reward.")


class TipSpeedCartCentreReward(unittest.TestCase):
    def factors(self, now):
        return SPEED_REWARD.score(inputs(now))[1]

    def test_target_speed_table(self):
        """__synthetic__/trainer/python/reward-plan.md section 2: 1 at or below the limit and outside
        the zone; passing straight up it falls towards the floor."""
        table = [
            (rig(), 0.0, 1.0),                                  # straight up, still
            (rig(speed=0.5), 0.0, 1.0),                         # at the limit
            (rig(speed=1.0), 0.5, 0.869),
            (rig(speed=-1.0), 0.5, 0.869),                      # either way round
            (rig(speed=2.0), 1.5, 0.741),
            (rig(speed=3.0), 2.5, 0.694),
            (rig(speed=5.25), 4.75, 0.670),
            (rig(speed=10.0), 9.5, 0.667),
            (rig(pole=math.pi / 2, speed=7.4), 0.0, 1.0),       # sideways, outside the zone
            (rig(pole=0.0, speed=10.5), 0.0, 1.0),              # passing the bottom
            (rig(pole=0.0), 0.0, 1.0),                          # hanging, still
        ]
        for now, over, score in table:
            f = self.factors(now)
            self.assertAlmostEqual(f["over_target"], over, places=12)
            self.assertAlmostEqual(f["target_speed"], score, places=3)

    def test_the_zone(self):
        """Section 2.1.2: d_x is the rail's part of the distance alone, it
        rides the cart, and a hanging pole's d_x of 0 is not in the zone."""
        for lean, x, charged in ((5.0, 0.039, True), (20.0, 0.154, True), (21.0, 0.161, False),
                                 (-20.0, 0.154, True), (-21.0, 0.161, False)):
            for cart in (0.0, 0.3):
                f = self.factors(rig(cart=cart, pole=math.pi + math.radians(lean), speed=3.0))
                self.assertAlmostEqual(f["d_x"], x, places=3)
                self.assertEqual(f["over_target"] > 0, charged, f"lean {lean}, cart {cart}")
        hanging = self.factors(rig(pole=0.0, speed=3.0))
        self.assertAlmostEqual(hanging["d_x"], 0.0, places=12)
        self.assertEqual(hanging["over_target"], 0.0)

    def test_speed_table(self):
        """Section 5: anywhere on the circle, with no zone."""
        table = [(12.0, 0.0, 1.0), (13.0, 1.0, 0.526), (14.0, 2.0, 0.352), (15.0, 3.0, 0.287), (20.0, 8.0, 0.250),
                 (33.0, 21.0, 0.250), (5.0, 0.0, 1.0)]
        for turning, over, score in table:
            for pole in (0.0, math.pi / 2, math.pi):
                for way in (1, -1):
                    f = self.factors(rig(pole=pole, speed=way * turning))
                    self.assertAlmostEqual(f["over_speed"], over, places=12)
                    self.assertAlmostEqual(f["speed"], score, places=3)

    def test_the_other_factors(self):
        """Sections 1, 3 and 4, the same tables as the other scheme's."""
        for now, tip in ((rig(), 1.0), (rig(cart=0.2), 1.0), (rig(pole=math.pi / 2), 0.042), (rig(pole=0.0), 0.011)):
            self.assertAlmostEqual(self.factors(now)["tip"], tip, places=3)
        for cart, score in ((0.03, 1.0), (0.1, 0.940), (-0.2, 0.850), (-0.44, 0.737)):
            self.assertAlmostEqual(self.factors(rig(cart=cart))["cart"], score, places=3)
        for now, score in ((rig(), 1.0), (rig(cart=0.2), 0.790), (rig(pole=0.0), 0.671)):
            self.assertAlmostEqual(self.factors(now)["reach"], score, places=3)

    def test_section_0_examples(self):
        """__synthetic__/trainer/python/reward-plan.md section 0, with the cart in the middle."""
        for now, score in ((rig(), 1.0), (rig(speed=10.0), 0.667), (rig(pole=math.pi / 2), 0.028),
                           (rig(pole=0.0), 0.0075), (rig(cart=0.2), 0.671)):
            self.assertAlmostEqual(SPEED_REWARD.score(inputs(now))[0], score, places=3)

    def test_both_cuts_multiply(self):
        """Section 5: at the floors, 0.667 x 0.250."""
        self.assertAlmostEqual(SPEED_REWARD.score(inputs(rig(speed=30.0)))[0], 0.667 * 0.250, places=6)

    def test_never_reaches_the_floors(self):
        """Squeezed, not clamped: every bit slower pays (sections 2.2 and 5.2)."""
        for factor, floor, start in (("target_speed", 0.667, 1), ("speed", 0.250, 13)):
            scores = [self.factors(rig(speed=start + x / 2))[factor] for x in range(40)]
            self.assertTrue(all(a > b for a, b in zip(scores, scores[1:])), factor)
            self.assertGreater(scores[-1], floor, factor)

    def test_straight_up_beats_any_lean(self):
        """Section 4's rule holds with the speed factors in the product."""
        for cart in (0.05, 0.2, 0.44, -0.2, -0.44):
            towards = 1 if cart > 0 else -1
            upright = SPEED_REWARD.score(inputs(rig(cart=cart)))[0]
            for tenths in range(1, 401):
                pole = math.pi + towards * math.radians(tenths / 10)
                self.assertLess(SPEED_REWARD.score(inputs(rig(cart=cart, pole=pole)))[0], upright)

    def test_product_range_and_commands(self):
        """r is the five factors' product in (0, 1], whatever the commands."""
        now, before = rig(cart=0.2, pole=3.0, speed=13.0), rig(cart=0.2, pole=2.8, speed=12.0)
        r, f = SPEED_REWARD.score(inputs(now, before))
        jumped, _ = SPEED_REWARD.score(inputs(now, before, command=20000, previous=-20000))
        self.assertEqual(r, jumped)
        self.assertAlmostEqual(r, f["tip"] * f["target_speed"] * f["cart"] * f["reach"] * f["speed"], places=15)
        self.assertTrue(0 < r <= 1)
        self.assertEqual(SPEED_REWARD.factors, ("d", "tip", "d_x", "over_target", "target_speed", "c_d", "cart", "d_r",
                                                "reach", "over_speed", "speed"))

    def test_table_checks(self):
        for factor, change, pattern in (("target_speed", {"zone_dx": -0.01}, r"reward\.target_speed\.zone_dx"),
                                        ("target_speed", {"limit": None}, r"reward\.target_speed.*limit"),
                                        ("target_speed", {"width": 0.0}, r"reward\.target_speed\.width"),
                                        ("target_speed", {"floor": 1.5}, r"reward\.target_speed\.floor"),
                                        ("speed", {"limit": -1.0}, r"reward\.speed\.limit"),
                                        ("speed", {"width": 0.0}, r"reward\.speed\.width"),
                                        ("speed", {"floor": None}, r"reward\.speed.*floor"),
                                        ("speed", {"zone_dx": 0.1}, r"reward\.speed: unknown")):
            changed = {k: v for k, v in {**SPEED_TABLES[factor], **change}.items() if v is not None}
            with self.assertRaisesRegex(ValueError, pattern):
                TipSpeedCartCentre.from_table({**SPEED_TABLES, factor: changed}, FACTS, "reward.")


class StrikeGapEnd(unittest.TestCase):
    END = StrikeGap(buffer=0.0136)

    def test_gaps(self):
        s = rig().sites()
        self.assertAlmostEqual(s.left_cart[0] - s.left_rail[0], 0.440)
        self.assertAlmostEqual(s.right_rail[0] - s.right_cart[0], 0.445)
        self.assertIsNone(self.END.check(inputs(rig())))

    def test_ends_within_b(self):
        self.assertIsNone(self.END.check(inputs(rig(cart=-0.440 + 0.014))))
        self.assertEqual(self.END.check(inputs(rig(cart=-0.440 + 0.013))), "left strike gap")
        self.assertEqual(self.END.check(inputs(rig(cart=0.445 - 0.013))), "right strike gap")

    def test_floor(self):
        """The cart top speed x the longest gap then the longest latency:
        0.9 m/s x (52 + 16) ms at 20-30 Hz and 8-15 ms."""
        with self.assertRaisesRegex(ValueError, "at least 0.06120"):
            StrikeGap.from_table({"buffer": 0.0611}, FACTS, "end.strike_gap.")
        self.assertEqual(StrikeGap.from_table({"buffer": 0.0613}, FACTS, "end.strike_gap.").buffer, 0.0613)


class ObservationAndAction(unittest.TestCase):
    def test_observation(self):
        observation = PoolAndCommands.from_table({"past_commands": 6}, FACTS, "observation.pool_and_commands.")
        self.assertEqual(observation.space().shape, (36,))
        self.assertEqual(list(inspect.signature(observation.build).parameters), ["pool", "history"])
        capture = support.fake(1, 0).next_captures()[0]
        values = observation.build(capture.pool, History.empty(6))
        self.assertTrue(observation.space().contains(values))
        self.assertEqual(list(values[30:]), [0.0] * 6)
        values = observation.build(capture.pool, History.empty(6).push(0.5, TOP // 2).push(-1.0, -TOP))
        self.assertEqual(list(values[30:32]), [-1.0, 0.5])

    def test_observation_ages_follow_the_slowest_command_rate(self):
        """The oldest age's bound is 5 of the longest gap: 50 ms at 20 Hz,
        rounded to the first 2 ms step later."""
        space = PoolAndCommands.from_table({"past_commands": 0}, FACTS, "observation.pool_and_commands.").space()
        self.assertAlmostEqual(float(space.low[0]), -5 * 52.0, places=4)

    def test_action(self):
        action = LinearRate.from_table({}, FACTS, "action.linear_rate.")
        for a, rate in ((-1.0, -TOP), (0.0, 0), (1.0, TOP), (2.0, TOP), (0.5, round(0.5 * TOP))):
            clipped, command = action.command([a], History.empty(1))
            self.assertEqual(command.rate, rate)
            self.assertEqual(clipped, max(-1.0, min(1.0, a)))


if __name__ == "__main__":
    unittest.main()
