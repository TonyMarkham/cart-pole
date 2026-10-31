"""The episode tracker's first-step rule, and the environment on the fake
link: its spaces, an end with its reason, a time limit as a truncation,
and SAC running on it with the step inputs stored at every row
(rl-rewrite-plan.md, build steps 6, 7 and 9)."""

import unittest

import numpy as np
from stable_baselines3 import SAC

import support
from buffer import InputsReplayBuffer
from contracts import Command
from env import LoopVecEnv
from episode import Episode


class FirstStep(unittest.TestCase):
    def test_capture_zero_is_not_scored(self):
        link = support.fake(1, 0)
        episode = Episode(6)
        with self.assertRaises(RuntimeError):
            episode.advance(0.0, Command(0), link.next_captures()[0])
        link = support.fake(1, 0)
        capture0 = link.next_captures()[0]
        episode.start(capture0)
        link.answer({0: __import__("link.answer", fromlist=["Answer"]).Answer.go(Command(1000))})
        capture1 = link.next_captures()[0]
        first = episode.advance(1000 / support.run_values().top_pulse_rate, Command(1000), capture1)
        self.assertEqual(first.previous, capture0.sites)
        self.assertEqual(first.current, capture1.sites)
        self.assertEqual((first.previous_pole, first.current_pole), (capture0.pole, capture1.pole))
        self.assertEqual((first.command, first.previous_command, first.step), (1000, 0, 0))
        second = episode.advance(0.0, Command(-500), capture1)
        self.assertEqual((second.command, second.previous_command, second.step), (-500, 1000, 1))


class Environment(unittest.TestCase):
    def setUp(self):
        self.fixture = support.Fixture(max_episode_steps=40)
        self.settings, self.gym = self.fixture.load()

    def tearDown(self):
        self.fixture.close()

    def env(self, instances=2, gym=None):
        """On a fake link opened with seed 0 and asked first, as a run starts."""
        link = support.fake(instances, 0)
        link.ask()
        return LoopVecEnv(gym or self.gym, link, instances, 0)

    def test_spaces_and_time_limit(self):
        env = self.env()
        observations = env.reset()
        self.assertEqual(observations.shape, (2, 60))
        for step in range(40):
            observations, rewards, dones, infos = env.step(np.zeros((2, 1), dtype=np.float32))
            for k in range(2):
                self.assertTrue(env.observation_space.contains(observations[k]))
                self.assertTrue(0 < rewards[k] <= 1)
        self.assertTrue(all(dones))
        self.assertTrue(all(info["TimeLimit.truncated"] for info in infos))
        self.assertEqual([info["end"] for info in infos], [None, None])
        # The new episodes' first observations hold no commands; the terminal ones were their own.
        self.assertEqual(list(observations[0][54:]), [0.0] * 6)
        self.assertEqual([info["episode_number"] for info in infos], [0, 1])
        self.assertEqual(env.episode, [2, 3])
        env.close()

    def test_end_with_reason(self):
        # Long enough for the full rate to close the right gap, 0.445 m less
        # b, at the tests' 0.9 m/s: about 11 steps of 41.5 ms.
        fixture = support.Fixture(max_episode_steps=100)
        try:
            _, gym = fixture.load()
        finally:
            fixture.close()
        env = self.env(1, gym)
        env.reset()
        for step in range(100):
            _, _, dones, infos = env.step(np.ones((1, 1), dtype=np.float32))  # full rate right
            if dones[0]:
                break
        self.assertEqual(infos[0]["end"], "right strike gap")
        self.assertFalse(infos[0]["TimeLimit.truncated"])
        self.assertIn("terminal_observation", infos[0])
        env.close()

    def test_a_command_past_the_top_pulse_rate_is_refused(self):
        """The command's bound is the run's top pulse rate (the run's values),
        checked before each answer is sent."""
        env = self.env(1)
        env.reset()
        top = self.gym.facts.top_pulse_rate

        class Past:
            history = 0

            def command(self, action, history):
                return 1.0, Command(top + 1)

        env.action = Past()
        with self.assertRaisesRegex(ValueError, f"at most \\+-{top} pulses/s"):
            env.step(np.ones((1, 1), dtype=np.float32))
        env.close()

    def test_sac_stores_inputs(self):
        env = self.env()
        model = SAC("MlpPolicy", env, seed=0, replay_buffer_class=InputsReplayBuffer, **self.gym.sac.arguments())
        model.learn(total_timesteps=200)
        buffer = model.replay_buffer
        rows = buffer.stored()
        self.assertEqual(rows, 100)
        from contracts import StepInputs
        for i in range(rows):
            for k in range(2):
                inputs = StepInputs.from_array(buffer.inputs[i, k])
                self.assertAlmostEqual(self.gym.reward.impl.score(inputs)[0], float(buffer.rewards[i, k]), places=6)
        env.close()


if __name__ == "__main__":
    unittest.main()
