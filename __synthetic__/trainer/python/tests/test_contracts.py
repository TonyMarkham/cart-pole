"""The contract types build from good values and refuse bad ones, each
refusal naming the field (rl-rewrite-plan.md, build step 2)."""

import unittest

import support  # noqa: F401  (__synthetic__/trainer/python/src on the path)
from contracts import Command, PoleState, Pool, RunValues, Sites, StepInputs
from link.fake import Rig

ROW = [0.0, -0.4, 0.082, 0.3, -0.368]


def rows(ages=(0, -10, -20, -30, -40, -50)):
    return [[float(a), *ROW[1:]] for a in ages]


class Contracts(unittest.TestCase):
    def test_pool(self):
        pool = Pool.from_rows(rows())
        self.assertEqual(len(pool.flat()), 30)
        with self.assertRaisesRegex(ValueError, "pool: 6 rows of 5"):
            Pool.from_rows(rows()[:5])
        with self.assertRaisesRegex(ValueError, "pool: ages run newest first"):
            Pool.from_rows(rows((0, -10, -10, -30, -40, -50)))
        with self.assertRaisesRegex(ValueError, "pool: a marker is within 1 m"):
            Pool.from_rows([[0.0, 1.5, *ROW[2:]], *rows()[1:]])

    def test_sites(self):
        sites = Rig().sites()
        self.assertEqual(Sites.from_flat(sites.flat()), sites)
        message = {role: list(getattr(sites, role)) for role in ("left", "right", "pivot", "end", "target",
                                                                 "dynamic_target", "cart", "cart_target",
                                                                 "left_rail", "left_cart", "right_rail",
                                                                 "right_cart")}
        self.assertEqual(Sites.from_message(message), sites)
        message["end"] = [0.0, 0.1]
        with self.assertRaisesRegex(ValueError, "sites: end is"):
            Sites.from_message(message)

    def test_pole(self):
        rig = Rig()
        rig.pole, rig.rate = 1.0, -2.5
        pole = rig.pole_state()
        self.assertEqual(PoleState.from_flat(pole.flat()), pole)
        message = {"speed": pole.speed, "angle": pole.angle, "com": list(pole.com), "hinge": list(pole.hinge)}
        self.assertEqual(PoleState.from_message(message), pole)
        for bad, pattern in (({**message, "com": [0.0, 0.1]}, "com is"),
                             ({**message, "hinge": [0.0, 0.1]}, "hinge is"),
                             ({**message, "speed": "fast"}, "speed is"),
                             ({**message, "angle": None}, "angle is"),
                             ({"speed": 0.0}, "exactly")):
            with self.assertRaisesRegex(ValueError, f"pole: .*{pattern}"):
                PoleState.from_message(bad)

    def test_command(self):
        """An i32, the link's type; the top pulse rate is the environment's
        check (test_episode_env)."""
        self.assertEqual(Command(2**31 - 1).rate, 2**31 - 1)
        self.assertEqual(Command(-(2**31)).rate, -(2**31))
        for rate in (2**31, -(2**31) - 1):
            with self.assertRaisesRegex(ValueError, "command: an i32"):
                Command(rate)
        with self.assertRaisesRegex(ValueError, "command: a whole number"):
            Command(True)

    def test_run_values(self):
        message = {"cart_top_speed": 0.36, "top_pulse_rate": 24000, "timestep": 0.002, "pole_mass": 0.402,
                   "pole_com_distance": 0.155, "pole_inertia": 0.0222, "gravity": 9.81}
        values = RunValues.from_message(message)
        self.assertEqual(values, RunValues(0.36, 24000, 0.002, 0.402, 0.155, 0.0222, 9.81))
        self.assertEqual(values.message(), message)
        for bad, pattern in (({**message, "top_pulse_rate": 0}, "top_pulse_rate"),
                             ({**message, "top_pulse_rate": 24000.0}, "top_pulse_rate"),
                             ({**message, "cart_top_speed": -0.36}, "cart_top_speed"),
                             ({**message, "timestep": 0}, "timestep"),
                             ({**message, "pole_inertia": 0}, "pole_inertia"),
                             ({**message, "gravity": -9.81}, "gravity"),
                             ({"cart_top_speed": 0.36, "timestep": 0.002}, "exactly")):
            with self.assertRaisesRegex(ValueError, f"run: .*{pattern}"):
                RunValues.from_message(bad)

    def test_the_real_links_answer(self):
        """The orchestrator's first line answers the request (link/real.py)."""
        from link.real import read_run
        line = ('{"run": {"cart_top_speed": 0.36, "top_pulse_rate": 24000, "timestep": 0.002, "pole_mass": 0.402, '
                '"pole_com_distance": 0.155, "pole_inertia": 0.0222, "gravity": 9.81}}\n')
        self.assertEqual(read_run(line), RunValues(0.36, 24000, 0.002, 0.402, 0.155, 0.0222, 9.81))
        for bad in ('{"pools": []}', '{"run": {"cart_top_speed": 0.36}}', "MuJoCo 3.15.0"):
            with self.assertRaises(ValueError):
                read_run(bad)

    def test_the_real_links_pools(self):
        """A pools line, the orchestrator's stdout line to the trainer
        (synthetic-loop-dia.md, arrow 17; read by link/real.py), carries each
        entry's fault, and each instance's id must come after its last: ids
        count every capture, and a reset drops the one or two taken after the
        last send, so they may skip (synthetic-loop-dia.md, Reset in place,
        Contracts, the fault)."""
        import json
        from contracts import SITE_ROLES
        from link.real import read_captures
        rig = Rig()
        sites = rig.sites()
        pole = rig.pole_state()

        def line(k, ident, fault, without=()):
            """One instance's pools line, with the keys in `without` left out."""
            entry = {"instance": k, "id": ident, "time": 0.032, "pool": rows(),
                     "sites": {role: list(getattr(sites, role)) for role in SITE_ROLES},
                     "pole": {"speed": pole.speed, "angle": pole.angle, "com": list(pole.com),
                              "hinge": list(pole.hinge)},
                     "fault": fault}
            for key in without:
                del entry[key]
            return json.dumps({"pools": [entry]})

        capture = read_captures(line(0, 7, False), {0: 4})[0]  # 5 and 6 dropped at a reset
        self.assertEqual((capture.id, capture.fault), (7, False))
        self.assertTrue(read_captures(line(0, 5, True), {0: 4})[0].fault)
        for bad, last in ((line(0, 4, False), 4), (line(0, 3, False), 4), (line(0, 5, False, without=("fault",)), 4),
                          (line(0, 5, "yes"), 4), (line(1, 5, False), 4)):
            with self.assertRaises(ValueError):
                read_captures(bad, {0: last})

    def test_step_inputs(self):
        rig = Rig()
        sites, before = rig.sites(), rig.pole_state()
        rig.pole, rig.rate = 0.5, 3.0
        after = rig.pole_state()
        inputs = StepInputs(sites, sites, 100, -50, 3, before, after)
        self.assertEqual(StepInputs.from_array(inputs.to_array()), inputs)
        with self.assertRaisesRegex(ValueError, "step inputs: step"):
            StepInputs(sites, sites, 0, 0, -1, before, after)


if __name__ == "__main__":
    unittest.main()
