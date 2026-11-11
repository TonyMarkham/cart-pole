"""The settings loader routes each table to its part, and refuses an
unknown plugin, a missing table and a value out of range, each naming the
key (rl-rewrite-plan.md, build step 3)."""

import sys
import tempfile
import unittest
from pathlib import Path

import support
from plugins import FOLDERS, available, load
from settings import Gym, Settings, read_train

REAL_GYM = Path(support.SRC).parent / "gym.toml"
REAL_SETTINGS = Path(support.SRC).parent / "settings.toml"


class Loader(unittest.TestCase):
    def setUp(self):
        self.fixture = support.Fixture()

    def tearDown(self):
        self.fixture.close()

    def gym_text(self) -> str:
        return (self.fixture.path / "gym.toml").read_text(encoding="utf-8")

    def refuses(self, old: str, new: str, pattern: str):
        self.fixture.write_gym()  # each refusal starts from the fixture as written
        path = self.fixture.path / "gym.toml"
        path.write_text(self.gym_text().replace(old, new), encoding="utf-8")
        from settings import Settings
        settings = Settings.load(self.fixture.path / "settings.toml")
        with self.assertRaisesRegex(ValueError, pattern):
            Gym.load(settings.gym, settings.facts(support.run_values()))

    def test_loads(self):
        settings, gym = self.fixture.load()
        self.assertEqual(settings.instances, 2)
        self.assertEqual(gym.facts.top_pulse_rate, 60_000)
        self.assertEqual(settings.command(0, 1)[-12:-4], ["--full-steps-per-rev", "200", "--microsteps", "32",
                                                          "--belt-um-per-rev", "96000", "--jump-limit", "1000000"])
        self.assertEqual([p.name for p in gym.parts()],
                         ["pool_and_commands", "linear_rate", "dm_control_cartpole"])
        self.assertEqual(gym.reward.impl.centered_margin, 2.0)
        self.assertEqual(gym.reward.table["centered"], {"margin": 2.0, "at_margin": 0.1, "floor": 0.5})

    def test_the_random_timings_reach_the_orchestrator(self):
        """settings.toml's bands and loss, and the gym file's
        max_episode_steps, are on the launch command as their flags, before
        --orin takes every argument after it (__documentation__/synthetic-loop-dia.md, Timing)."""
        settings, _ = self.fixture.load()
        command = settings.command(0, 1)
        flags = {"--frame-ready-min": "43.0", "--frame-ready-max": "48.0", "--to-uno-min": "20.5",
                 "--to-uno-max": "27.8", "--change-motor-min": "3.6", "--change-motor-max": "123.3",
                 "--next-frame-ready-min": "0.0", "--next-frame-ready-max": "16.7",
                 "--drop-one-message-in": "3000", "--max-episode-steps": "60"}
        for flag, value in flags.items():
            self.assertEqual(command[command.index(flag) + 1], value, flag)
            self.assertLess(command.index(flag), command.index("--orin"), flag)
        for gone in ("--rnd-delay-min", "--rnd-delay-max", "--command-rate-min", "--command-rate-max"):
            self.assertNotIn(gone, command)
        # The bands' longest and shortest gaps between captures, by hand:
        # 48 + 27.8 + 16.7 - 43 = 49.5 ms and 43 + 20.5 + 0 - 48 = 15.5 ms.
        self.assertAlmostEqual(settings.timings.longest_capture_gap_ms(), 49.5)
        self.assertAlmostEqual(settings.timings.shortest_capture_gap_ms(), 15.5)

    def test_a_bad_timing_is_refused(self):
        """Each band 0 <= min <= max, and the loss a whole number from 1, each
        refusal naming its key."""
        path = self.fixture.path / "settings.toml"
        text = path.read_text(encoding="utf-8")
        for old, new, pattern in (("to_uno_min = 20.5", "to_uno_min = 30.0", r"to_uno_min 30\.0 is above"),
                                  ("change_motor_min = 3.6", "change_motor_min = -1.0", r"change_motor_min"),
                                  ("drop_one_message_in = 3000", "drop_one_message_in = 0",
                                   r"drop_one_message_in"),
                                  ("next_frame_ready_max = 16.7\n", "", r"next_frame_ready_max")):
            path.write_text(text.replace(old, new), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, pattern):
                Settings.load(path)
        path.write_text(text, encoding="utf-8")

    def test_the_grid_and_the_screen_reach_the_orchestrator(self):
        """The whole group's launch has the settings' grid (the fixture's 1 x 2);
        env_check's one instance is a 1 x 1 grid, on the same screen."""
        settings, _ = self.fixture.load()

        def flags(instances):
            command = settings.command(0, instances)
            return [command[command.index(flag) + 1]
                    for flag in ("--instance-rows", "--instance-columns", "--screen-width", "--screen-height")]

        self.assertEqual(flags(2), ["1", "2", "3840", "2112"])
        self.assertEqual(flags(1), ["1", "1", "3840", "2112"])
        with self.assertRaisesRegex(ValueError, r"runs 2 instances or 1, got 3"):
            settings.command(0, 3)

    def test_the_grid_holds_at_most_twelve(self):
        from settings import Settings
        path = self.fixture.path / "settings.toml"
        text = path.read_text(encoding="utf-8")
        path.write_text(text.replace("instance_rows = 1", "instance_rows = 4").replace(
            "instance_columns = 2", "instance_columns = 3"), encoding="utf-8")
        self.assertEqual(Settings.load(path).instances, 12)
        path.write_text(text.replace("instance_rows = 1", "instance_rows = 13"), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, r"instance_rows"):
            Settings.load(path)
        path.write_text(text.replace("instance_rows = 1", "instance_rows = 7"), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, r"is 14 instances; at most 12"):
            Settings.load(path)

    def test_unknown_plugin(self):
        self.refuses('scheme = "dm_control_cartpole"', 'scheme = "nothing"', r"reward: no reward plugin 'nothing'")

    def test_missing_table(self):
        self.refuses("[end]\nplugins", "[ending]\nplugins", r"unknown key")
        self.refuses('[env]\nmax_episode_steps = 60', '', r"env: a table is required")

    def test_out_of_range(self):
        self.refuses("[reward.small_control]\nmargin = 1.0", "[reward.small_control]\nmargin = 0",
                     r"reward\.small_control\.margin: in \(0\.0")
        self.refuses("margin = 1.0\nfloor = 0.8", "margin = 1.0\nfloor = 1.5", r"reward\.small_control\.floor")
        self.refuses("margin = 5.0\n", "", r"reward\.small_velocity.*margin")
        self.refuses("margin = 5.0\nat_margin = 0.1", "margin = 5.0\nat_margin = 0", r"reward\.small_velocity\.at_margin")
        self.refuses("margin = 5.0\nat_margin = 0.1", "margin = 5.0\nat_margin = 1.5",
                     r"reward\.small_velocity\.at_margin")
        self.refuses("[reward.centered]\nmargin = 2.0", "[reward.centered]\nmargin = -1", r"reward\.centered\.margin")
        self.refuses("[reward.centered]", "[reward.centereds]", r"reward\.centered.*margin")
        # The command rate and the strike-gap end are gone (__documentation__/synthetic-loop-dia.md,
        # Timing, The rail's ends): a gym file that still names them is refused.
        self.refuses("max_episode_steps = 60", "command_rate_min = 20.0\nmax_episode_steps = 60",
                     r"env: unknown key\(s\) \['command_rate_min'\]")
        self.refuses("plugins = []", 'plugins = ["strike_gap"]', r"end: no end plugin 'strike_gap'")
        self.refuses("past_commands = 6", "past_commands = -1", r"observation\.pool_and_commands\.past_commands")
        self.refuses("age_unit_ms = 1000.0", "age_unit_ms = 0", r"observation\.pool_and_commands\.age_unit_ms")
        self.refuses('ent_coef = "auto_0.1"', 'ent_coef = "auto_"', r"sac\.ent_coef")
        self.refuses('ent_coef = "auto_0.1"', 'ent_coef = "auto_-1"', r"sac\.ent_coef")
        self.refuses('ent_coef = "auto_0.1"', 'ent_coef = "manual"', r"sac\.ent_coef")
        self.refuses('ent_coef = "auto_0.1"', "ent_coef = 0", r"sac\.ent_coef")
        self.refuses('ent_coef = "auto_0.1"', "", r"sac\.ent_coef: required")

    def test_typo_in_plugin_table(self):
        self.refuses("margin = 1.0\nfloor = 0.8", "margin = 1.0\nfloor = 0.8\nzone = 1",
                     r"reward\.small_control: unknown key\(s\) \['zone'\]")

    def test_real_gym_loads(self):
        """__synthetic__/trainer/python/gym.toml as committed loads on MuJoCo's run values for the
        rig (the built orchestrator's --run-values), and
        settings.toml's [drive] holds the jump limit the user set on
        2026-11-07, 900,000, below the 1,210,937 measured
        (jump-limit-results.md), which the orchestrator is passed."""
        settings = Settings.load(REAL_SETTINGS)
        gym = Gym.load(settings.gym, settings.facts(support.rig_values(settings)))
        self.assertEqual(gym.reward.name, "dm_control_cartpole")
        self.assertEqual(gym.sac.arguments()["ent_coef"], "auto_0.1")
        self.assertEqual(settings.drive.jump_limit, 900_000)
        self.assertIn("--jump-limit", settings.command(0, 1))

    def test_train_alone(self):
        """[train] reads without the run's values: a training run needs its
        folder before the launch."""
        train = read_train(self.fixture.path / "gym.toml")
        self.assertEqual(train.runs_dir, (self.fixture.path / "runs").resolve())
        self.assertEqual(read_train(REAL_GYM).runs_dir, (REAL_GYM.parent / "runs").resolve())

    def test_drive_is_required(self):
        from settings import Settings
        path = self.fixture.path / "settings.toml"
        path.write_text(path.read_text(encoding="utf-8").replace("microsteps = 32", "microsteps = 0"),
                        encoding="utf-8")
        with self.assertRaisesRegex(ValueError, r"drive\.microsteps"):
            Settings.load(path)

    def test_a_second_plugin_loads_by_name(self):
        """Swapping a scheme is a new file and a name: proven with a trivial
        plugin that is not in the repo (rl-rewrite-plan.md, build step 4)."""
        import rewards
        with tempfile.TemporaryDirectory() as folder:
            Path(folder, "always_one.py").write_text(
                "class AlwaysOne:\n"
                "    factors = ()\n"
                "    @staticmethod\n"
                "    def from_table(table, facts, where):\n"
                "        return AlwaysOne()\n"
                "    def score(self, inputs):\n"
                "        return 1.0, {}\n"
                "PLUGIN = AlwaysOne\n", encoding="utf-8")
            rewards.__path__.append(folder)
            try:
                self.assertIn("always_one", available("reward"))
                part = load("reward", "always_one", {}, "reward", support.facts())
                self.assertEqual(part.impl.score(None), (1.0, {}))
            finally:
                rewards.__path__.remove(folder)
                sys.modules.pop("rewards.always_one", None)
        self.assertEqual(FOLDERS["reward"], "rewards")


if __name__ == "__main__":
    unittest.main()
