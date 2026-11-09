"""The plugins against hand-computed values (rl-rewrite-plan.md, build
steps 4 and 5): the reward against __synthetic__/trainer/python/dm_control_reward.md's tables, the
longest gap between captures on the grid, the observation's shape and
contents, the action's rounding and bound."""

import inspect
import math
import unittest

import support
from contracts import StepInputs
from link.fake import Rig
from observations.pool_and_commands import PoolAndCommands
from actions.linear_rate import LinearRate
from plugins import History, load
from rewards.dm_control_cartpole import DmControlCartpole

FACTS = support.facts()
TOP = FACTS.top_pulse_rate  # 60,000 pulses/s at the tests' run values
DM_TABLES = {"small_control": {"margin": 1.0, "floor": 0.8},
             "small_velocity": {"margin": 5.0, "at_margin": 0.1, "floor": 0.5},
             "centered": {"margin": 2.0, "at_margin": 0.1, "floor": 0.5}}
DM_REWARD = DmControlCartpole.from_table(DM_TABLES, FACTS, "reward.")


def rig(cart=0.0, pole=math.pi, speed=0.0):
    """The fixture rig: pole 0 hanging, pi upright, turning at `speed` rad/s."""
    r = Rig()
    r.cart, r.pole, r.rate = cart, pole, speed
    return r


def inputs(now, before=None, command=0, previous=0):
    before = before or now
    return StepInputs(before.sites(), now.sites(), command, previous, 0, before.pole_state(), now.pole_state())


class DmControlCartpoleReward(unittest.TestCase):
    """Against the tables in __synthetic__/trainer/python/dm_control_reward.md, DeepMind's own
    numbers, worked from its formulas."""

    def factors(self, now, command=0):
        return DM_REWARD.score(inputs(now, command=command))[1]

    def test_section_0_table(self):
        """Section 0's table. The fixture's cart runs past this rig's rail for the
        1 m and 1.8 m rows, which are DeepMind's."""
        lean = math.pi - math.radians(10)
        table = [
            (rig(), 0, 1.0),
            (rig(cart=1.0), 0, 0.781),
            (rig(), TOP, 0.8),
            (rig(speed=5.0), 0, 0.55),
            (rig(cart=1.0, speed=5.0), TOP, 0.344),
            (rig(pole=lean), 0, 0.992),
            (rig(pole=math.pi / 2), 0, 0.5),
            (rig(pole=0.0), 0, 0.0),
            (rig(cart=1.8, pole=0.0, speed=10.0), TOP, 0.0),
        ]
        for now, command, score in table:
            self.assertAlmostEqual(DM_REWARD.score(inputs(now, command=command))[0], score, places=3)

    def test_upright_table(self):
        """Section 1: the pole's angle alone, either side the same, wherever the cart is."""
        table = [(0.0, 1.0, 1.0), (10.0, 0.985, 0.992), (45.0, 0.707, 0.854), (90.0, 0.0, 0.5), (135.0, -0.707, 0.146),
                 (180.0, -1.0, 0.0)]
        for degrees, cos_theta, upright in table:
            for way in (1, -1):
                for cart in (0.0, 0.3):
                    f = self.factors(rig(cart=cart, pole=math.pi + way * math.radians(degrees)))
                    self.assertAlmostEqual(f["cos_theta"], cos_theta, places=3)
                    self.assertAlmostEqual(f["upright"], upright, places=3)

    def test_a_pole_past_a_full_turn_scores_the_same(self):
        """MuJoCo's angle counts on past a full turn (synthetic-loop-dia.md, the pole)."""
        self.assertAlmostEqual(self.factors(rig(pole=math.pi + 4 * math.pi))["upright"], 1.0, places=12)

    def test_small_control_table(self):
        """Section 2: u is the command over the top pulse rate; either way the same."""
        for fraction, score in ((0.0, 1.0), (0.25, 0.9875), (0.5, 0.95), (0.75, 0.8875), (1.0, 0.8)):
            for way in (1, -1):
                f = self.factors(rig(), command=way * round(fraction * TOP))
                self.assertAlmostEqual(f["u"], way * fraction, places=12)
                self.assertAlmostEqual(f["small_control"], score, places=12)

    def test_small_control_stops_at_its_floor_past_the_margin(self):
        """Section 2.1.1: the quadratic is cut off at 0, so it never turns back up."""
        tables = {**DM_TABLES, "small_control": {"margin": 0.5, "floor": 0.8}}
        reward = DmControlCartpole.from_table(tables, FACTS, "reward.")
        self.assertEqual(reward.score(inputs(rig(), command=TOP))[1]["small_control"], 0.8)

    def test_small_velocity_table(self):
        """Section 3: with no zone, at the top and the bottom alike."""
        table = [(0.0, 1.0), (1.0, 0.956), (2.0, 0.846), (3.0, 0.718), (5.0, 0.55), (7.0, 0.505), (15.0, 0.5)]
        for omega, score in table:
            for pole in (0.0, math.pi):
                for way in (1, -1):
                    self.assertAlmostEqual(self.factors(rig(pole=pole, speed=way * omega))["small_velocity"], score,
                                           places=3)

    def test_centered_table(self):
        """Section 4, from |Cart - Cart Target|; left and right the same. 0.44 m is
        section 4.1.2's figure for this rig's rail."""
        table = [(0.0, 1.0), (0.25, 0.982), (0.44, 0.947), (0.5, 0.933), (1.0, 0.781), (1.5, 0.637), (1.8, 0.577),
                 (2.0, 0.55)]
        for x, score in table:
            for way in (1, -1):
                f = self.factors(rig(cart=way * x))
                self.assertAlmostEqual(f["x"], x, places=12)
                self.assertAlmostEqual(f["centered"], score, places=3)

    def test_the_product(self):
        """r is exactly the four scores' product, and only this step's command counts."""
        now = rig(cart=0.2, pole=2.5, speed=3.0)
        r, f = DM_REWARD.score(inputs(now, command=TOP // 2, previous=-TOP))
        self.assertAlmostEqual(r, f["upright"] * f["small_control"] * f["small_velocity"] * f["centered"], places=15)
        self.assertEqual(r, DM_REWARD.score(inputs(now, command=TOP // 2, previous=TOP))[0])
        self.assertEqual(DM_REWARD.factors, tuple(f))

    def test_it_loads_by_name_from_its_tables_alone(self):
        """The plugin pattern: the other schemes' tables beside it are ignored."""
        part = load("reward", "dm_control_cartpole", {**DM_TABLES, "other_scheme": {"width": 0.2}}, "reward", FACTS)
        self.assertEqual(part.impl, DM_REWARD)
        self.assertEqual(set(part.table), set(DM_TABLES))

    def test_the_stored_step_rescores_the_same(self):
        """A resume rescores stored steps from their flat form (buffer.py)."""
        step = inputs(rig(cart=0.2, pole=2.5, speed=3.0), command=TOP // 2)
        self.assertEqual(DM_REWARD.score(StepInputs.from_array(step.to_array()))[0], DM_REWARD.score(step)[0])

    def test_table_checks(self):
        for factor, change, pattern in (("small_control", {"margin": 0.0}, r"reward\.small_control\.margin"),
                                        ("small_control", {"floor": 1.5}, r"reward\.small_control\.floor"),
                                        ("small_control", {"floor": None}, r"reward\.small_control.*floor"),
                                        ("small_velocity", {"margin": 0.0}, r"reward\.small_velocity\.margin"),
                                        ("small_velocity", {"at_margin": 0.0}, r"reward\.small_velocity\.at_margin"),
                                        ("small_velocity", {"at_margin": 1.5}, r"reward\.small_velocity\.at_margin"),
                                        ("centered", {"margin": -1.0}, r"reward\.centered\.margin"),
                                        ("centered", {"floor": None}, r"reward\.centered.*floor"),
                                        ("centered", {"zone": 0.05}, r"reward\.centered: unknown")):
            changed = {k: v for k, v in {**DM_TABLES[factor], **change}.items() if v is not None}
            with self.assertRaisesRegex(ValueError, pattern):
                DmControlCartpole.from_table({**DM_TABLES, factor: changed}, FACTS, "reward.")


class LongestGap(unittest.TestCase):
    def test_the_longest_gap_sits_on_the_grid(self):
        """Each capture is rounded to the nearest 2 ms timestep, so a gap moves
        by less than 2 ms, and every gap is below the bands' longest, 49.5 ms
        today: the longest gap on the grid is 50 ms. A gap under exactly 50 ms
        still rounds to at most 50; one under 50.1 ms can reach 52."""
        self.assertEqual(support.facts(49.5).longest_gap_ms(), 50.0)
        self.assertEqual(support.facts(50.0).longest_gap_ms(), 50.0)
        self.assertEqual(support.facts(50.1).longest_gap_ms(), 52.0)
        self.assertEqual(support.facts(15.5).longest_gap_ms(), 16.0)


class ObservationAndAction(unittest.TestCase):
    def test_observation(self):
        observation = PoolAndCommands.from_table({"past_commands": 6, "age_unit_ms": 1000.0}, FACTS,
                                                 "observation.pool_and_commands.")
        self.assertEqual(observation.space().shape, (36,))
        self.assertEqual(list(inspect.signature(observation.build).parameters), ["pool", "history"])
        capture = support.fake(1, 0).next_captures()[0]
        values = observation.build(capture.pool, History.empty(6))
        self.assertTrue(observation.space().contains(values))
        self.assertEqual(list(values[30:]), [0.0] * 6)
        values = observation.build(capture.pool, History.empty(6).push(0.5, TOP // 2).push(-1.0, -TOP))
        self.assertEqual(list(values[30:32]), [-1.0, 0.5])

    def test_observation_ages_follow_the_longest_gap(self):
        """The oldest age's bound is 5 of the longest gap between captures: 50
        ms on the grid for today's 49.5 ms; 0.25 of the 1,000 ms unit."""
        space = PoolAndCommands.from_table({"past_commands": 0, "age_unit_ms": 1000.0}, FACTS,
                                           "observation.pool_and_commands.").space()
        self.assertAlmostEqual(float(space.low[0]), -5 * 50.0 / 1000, places=4)

    def test_observation_divides_every_age_and_nothing_else(self):
        """Each entry's first float is its age, and a unit of 1,000 divides only
        those: the markers and the commands stay as they were."""
        table = {"past_commands": 2, "age_unit_ms": 1000.0}
        observation = PoolAndCommands.from_table(table, FACTS, "observation.pool_and_commands.")
        capture = support.fake(1, 0).next_captures()[0]
        raw = capture.pool.flat()
        values = observation.build(capture.pool, History.empty(2).push(0.5, TOP // 2))
        self.assertTrue(observation.space().contains(values))
        for i, value in enumerate(raw):
            expected = value / 1000 if i % 5 == 0 else value
            self.assertAlmostEqual(float(values[i]), expected, places=5)
        self.assertEqual(float(values[30]), 0.5)
        self.assertLess(float(values[25]), 0.0)  # the oldest entry's age, not left at 0

    def test_observation_refuses_a_bad_unit(self):
        for unit in (0.0, -1000.0):
            with self.assertRaisesRegex(ValueError, "observation.pool_and_commands.age_unit_ms"):
                PoolAndCommands.from_table({"past_commands": 0, "age_unit_ms": unit}, FACTS,
                                           "observation.pool_and_commands.")

    def test_action(self):
        action = LinearRate.from_table({}, FACTS, "action.linear_rate.")
        for a, rate in ((-1.0, -TOP), (0.0, 0), (1.0, TOP), (2.0, TOP), (0.5, round(0.5 * TOP))):
            clipped, command = action.command([a], History.empty(1))
            self.assertEqual(command.rate, rate)
            self.assertEqual(clipped, max(-1.0, min(1.0, a)))


if __name__ == "__main__":
    unittest.main()
