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
                         ["pool_and_commands", "linear_rate", "tip_speed_cart_centre", "strike_gap"])
        self.assertEqual(gym.reward.impl.width, 0.2)
        self.assertEqual(gym.reward.table["cart"], {"zone": 0.05, "width": 0.25, "floor": 0.667})

    def test_the_command_rates_reach_the_orchestrator(self):
        """The gym file's [env] rates are on the launch command, before --orin
        takes every argument after it; the Orin is not told them."""
        settings, gym = self.fixture.load()
        command = settings.command(0, 1)
        self.assertEqual(command[command.index("--command-rate-min") + 1], "20.0")
        self.assertEqual(command[command.index("--command-rate-max") + 1], "30.0")
        self.assertLess(command.index("--command-rate-max"), command.index("--orin"))
        self.assertEqual((gym.command_rates.low_hz, gym.command_rates.high_hz), (20.0, 30.0))

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
        self.refuses('scheme = "tip_speed_cart_centre"', 'scheme = "nothing"', r"reward: no reward plugin 'nothing'")

    def test_missing_table(self):
        self.refuses("[end]\nplugins", "[ending]\nplugins", r"unknown key")
        self.refuses('[env]\ncommand_rate_min = 20.0\ncommand_rate_max = 30.0\nmax_episode_steps = 60', '',
                     r"env: a table is required")

    def test_out_of_range(self):
        self.refuses("[reward.tip]\nwidth = 0.2", "[reward.tip]\nwidth = 0", r"reward\.tip\.width: in \(0\.0")
        self.refuses("zone_dx = 0.154", "zone_dx = -0.01", r"reward\.target_speed\.zone_dx")
        self.refuses("zone_dx = 0.154\n", "", r"reward\.target_speed.*zone_dx")
        self.refuses("limit = 12.0", "limit = -1", r"reward\.speed\.limit")
        self.refuses("zone = 0.050", "zone = -0.01", r"reward\.cart\.zone")
        self.refuses("[reward.reach]\nwidth = 0.2", "[reward.reach]\nwidth = 0", r"reward\.reach\.width: in \(0\.0")
        self.refuses("[reward.reach]\nwidth = 0.2\nfloor = 0.667", "[reward.reach]\nwidth = 0.2\nfloor = 1.5",
                     r"reward\.reach\.floor")
        self.refuses("[reward.cart]", "[reward.carts]", r"reward\.cart.*zone")
        self.refuses("buffer = 0.062", "buffer = 0.01", r"end\.strike_gap\.buffer: at least")
        self.refuses("command_rate_min = 20.0", "command_rate_min = 0.0", r"env\.command_rate_min")
        self.refuses("command_rate_min = 20.0", "command_rate_min = 40.0", r"command_rate_min 40\.0 is above")
        self.refuses("past_commands = 6", "past_commands = -1", r"observation\.pool_and_commands\.past_commands")

    def test_typo_in_plugin_table(self):
        self.refuses("zone_dx = 0.154", "zone_dx = 0.154\nzone = 1", r"reward\.target_speed: unknown key\(s\) \['zone'\]")

    def test_real_gym_loads(self):
        """__synthetic__/trainer/python/gym.toml as committed loads on MuJoCo's run values for the
        rig (the built orchestrator's --run-values), and
        settings.toml's [drive] holds the measured jump limit
        (jump-limit-results.md), which the orchestrator is passed."""
        settings = Settings.load(REAL_SETTINGS)
        gym = Gym.load(settings.gym, settings.facts(support.rig_values(settings)))
        self.assertEqual(gym.reward.name, "tip_speed_cart_centre")
        self.assertEqual(settings.drive.jump_limit, 1_210_937)
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
