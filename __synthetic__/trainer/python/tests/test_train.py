"""The training run on the fake link: the run folder and its manifest, a
resume that rescores, the resumes that are refused, the replay of a
recorded session and the measures over it (rl-rewrite-plan.md, build
steps 8, 9 and 10)."""

import contextlib
import io
import json
import unittest

import numpy as np
from stable_baselines3.common.save_util import load_from_pkl

import support
import train
from contracts import StepInputs
from settings import Gym
from env import LoopVecEnv
from link.fake import G, POLE_COM, POLE_INERTIA, POLE_MASS
from link.replay import ReplayLink
from measures import actor_spread, entropy, episodes
from record import Record
from run_folder import RunFolder


def quiet(function, *args, **kwargs):
    with contextlib.redirect_stdout(io.StringIO()) as out:
        result = function(*args, **kwargs)
    return result, out.getvalue()


class TrainingRun(unittest.TestCase):
    def setUp(self):
        self.fixture = support.Fixture(max_episode_steps=40)

    def tearDown(self):
        self.fixture.close()

    def session(self, resume=None):
        settings, gym = self.fixture.load()
        code, out = quiet(train.train, settings, resume, link=support.fake_link)
        self.assertEqual(code, 0, out)
        return gym

    def folder(self) -> RunFolder:
        return RunFolder.latest(self.fixture.path / "runs")

    def test_run_folder_and_manifest(self):
        self.session()
        folder = self.folder()
        manifest = folder.read_manifest()
        self.assertEqual(manifest["steps"], 200)
        self.assertEqual(manifest["plugins"]["reward"], {"name": "dm_control_cartpole",
                                                         "table": {"small_control": {"margin": 1.0, "floor": 0.8},
                                                                   "small_velocity": {"margin": 5.0, "at_margin": 0.1,
                                                                                      "floor": 0.5},
                                                                   "centered": {"margin": 2.0, "at_margin": 0.1,
                                                                                "floor": 0.5}}})
        self.assertEqual([p["name"] for p in manifest["plugins"]["end"]], ["strike_gap"])
        self.assertEqual(manifest["run_values"], {"cart_top_speed": 0.9, "top_pulse_rate": 60000, "timestep": 0.002,
                                                  "pole_mass": POLE_MASS, "pole_com_distance": POLE_COM,
                                                  "pole_inertia": POLE_INERTIA, "gravity": G})
        self.assertIn("commit", manifest["git"])
        self.assertIn("modified", manifest["mjcf"])
        self.assertIn("past_commands = 6", manifest["gym_toml"])
        for name in ("latest.zip", "latest_buffer.pkl", "model_100.zip", "model_100.json", "model_200.zip",
                     "episodes.csv", "captures_from_0.csv", "settings_from_0.toml", "gym_from_0.toml"):
            self.assertTrue((folder.path / name).is_file(), name)

    def test_resume_rescores(self):
        self.session()
        self.fixture.write_gym(centered_margin=0.5, total_steps=300)
        gym = self.session(train.LATEST)
        buffer = load_from_pkl(self.folder().buffer)
        rows = buffer.stored()
        self.assertEqual(rows, 150)
        for i in range(rows):
            for k in range(buffer.n_envs):
                expected = gym.reward.impl.score(StepInputs.from_array(buffer.inputs[i, k]))[0]
                self.assertAlmostEqual(float(buffer.rewards[i, k]), expected, places=6)
        self.assertTrue((self.folder().path / "captures_from_200.csv").is_file())

    def test_resume_refusals(self):
        self.session()
        self.fixture.write_gym(buffer=0.07, total_steps=300)
        settings, gym = self.fixture.load()
        with self.assertRaisesRegex(SystemExit, "end conditions"):
            quiet(train.train, settings, train.LATEST, link=support.fake_link)
        self.fixture.write_gym(past_commands=4, total_steps=300)
        settings, gym = self.fixture.load()
        with self.assertRaisesRegex(SystemExit, "observation"):
            quiet(train.train, settings, train.LATEST, link=support.fake_link)
        # Other command rates: each stored action was held for another time.
        self.fixture.write_gym(total_steps=300)
        path = self.fixture.path / "gym.toml"
        path.write_text(path.read_text(encoding="utf-8").replace("command_rate_max = 30.0", "command_rate_max = 25.0"),
                        encoding="utf-8")
        settings, gym = self.fixture.load()
        with self.assertRaisesRegex(SystemExit, "command rates"):
            quiet(train.train, settings, train.LATEST, link=support.fake_link)
        # Other run values (a re-exported MJCF): the action would mean other pulse rates.
        self.fixture.write_gym(total_steps=300)
        settings, gym = self.fixture.load()
        other = lambda *a: support.fake_link(*a, cart_top_speed=0.72)  # noqa: E731
        with self.assertRaisesRegex(SystemExit, "run's values"):
            quiet(train.train, settings, train.LATEST, link=other)

    def test_replay_and_measures(self):
        self.session()
        folder = self.folder()
        settings, gym = self.fixture.load()
        # The recorded session, replayed with its own actions, gives the same rewards.
        recorded = folder.captures(0)
        replay = ReplayLink(recorded)
        values = replay.ask()  # the run's values, from the run's manifest
        self.assertEqual(values, support.run_values())
        gym = Gym.load(settings.gym, settings.facts(values))
        again = Record(folder.path / "replayed.csv", gym.reward.impl.factors)
        env = LoopVecEnv(gym, replay, replay.instances, settings.seed, record=again)
        env.reset()
        while all(replay.remaining(k) for k in range(env.num_envs)):
            actions = [replay.recorded_action(k) for k in range(env.num_envs)]
            if None in actions:
                break
            env.step(np.array([[a] for a in actions], dtype=np.float32))
        env.close()
        again.close()
        original = recorded.read_text(encoding="utf-8").splitlines()
        replayed = (folder.path / "replayed.csv").read_text(encoding="utf-8").splitlines()
        self.assertGreater(len(replayed), 100)
        self.assertEqual(replayed, original[: len(replayed)])
        # Each measure prints its table from the run folder.
        tables = __import__("tomllib").loads((self.fixture.path / "measures.toml").read_text())
        _, table = quiet(episodes.run, folder, tables["episodes"])
        self.assertIn("| Steps | Episodes |", table)
        self.assertIn("| 0–100 |", table)
        _, table = quiet(entropy.run, folder, {})
        self.assertIn("| 200 |", table)
        _, table = quiet(actor_spread.run, folder, tables["actor_spread"])
        self.assertIn("| 100 |", table)


if __name__ == "__main__":
    unittest.main()
