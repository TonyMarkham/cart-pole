"""Checks the policy's arithmetic with a network small enough to work out by hand,
and the shipped policy file against the trainer's own answers. Needs no camera,
GPU or PyTorch.

usage (from __live__/orin/python): uv run python src/test_policy.py
"""

import tempfile
import unittest
from pathlib import Path

import numpy as np

from policy import ENTRIES, ROW, Policy, PolicyError

SHIPPED = Path(__file__).resolve().parent.parent / "saved" / "policy.npz"
INPUTS = ENTRIES * ROW  # past_commands 0 in the hand-made network


def hand_made(path: Path, **changes) -> None:
    """Two hidden numbers, relu(x0) and relu(-x0), passed straight through, then
    their difference: mean = x0 whatever its sign, so a = tanh(x0)."""
    w1 = np.zeros((2, INPUTS))
    w1[0, 0], w1[1, 0] = 1.0, -1.0
    values = {
        "hidden_1_weight": w1, "hidden_1_bias": np.zeros(2),
        "hidden_2_weight": np.eye(2), "hidden_2_bias": np.zeros(2),
        "output_weight": np.array([[1.0, -1.0]]), "output_bias": np.zeros(1),
        "top_pulse_rate": np.int64(44_600), "age_unit_ms": np.float64(1_000.0), "past_commands": np.int64(0),
        "command_rate_min_hz": np.float64(20.0), "command_rate_max_hz": np.float64(30.0),
        "check_inputs": np.zeros((1, INPUTS)), "check_actions": np.zeros(1), "source": np.str_("hand-made"),
    }
    np.savez(path, **{**values, **changes})


class HandMade(unittest.TestCase):
    def setUp(self) -> None:
        self.folder = tempfile.TemporaryDirectory()
        self.path = Path(self.folder.name) / "policy.npz"

    def tearDown(self) -> None:
        self.folder.cleanup()

    def test_the_best_guess_is_tanh_of_the_mean(self) -> None:
        hand_made(self.path)
        policy = Policy.load(self.path)
        inputs = np.zeros((2, INPUTS))
        inputs[0, 0], inputs[1, 0] = 0.5, -0.5
        # tanh(0.5) = 0.46211715726
        np.testing.assert_allclose(policy.action(inputs), [0.46211715726, -0.46211715726], atol=1e-10)

    def test_the_command_is_a_times_the_top_rate(self) -> None:
        hand_made(self.path)
        policy = Policy.load(self.path)
        # 0.46211715726 x 44,600 = 20,610.4
        self.assertEqual(policy.command(0.46211715726), 20_610)
        self.assertEqual(policy.command(-1.0), -44_600)
        self.assertEqual(policy.command(1.5), 44_600)

    def test_the_check_compares_with_the_trainers_answers(self) -> None:
        inputs = np.zeros((1, INPUTS))
        inputs[0, 0] = 0.5
        hand_made(self.path, check_inputs=inputs, check_actions=np.array([0.4621]))
        self.assertAlmostEqual(Policy.load(self.path).check(), 0.00001715726, places=10)

    def test_a_network_reading_the_wrong_inputs_is_refused(self) -> None:
        hand_made(self.path, past_commands=np.int64(6))
        with self.assertRaisesRegex(PolicyError, "reads 30 inputs and gives 1, not 36 and 1"):
            Policy.load(self.path)

    def test_a_missing_file_says_how_to_make_it(self) -> None:
        with self.assertRaisesRegex(PolicyError, "export_policy.py"):
            Policy.load(self.path)


@unittest.skipUnless(SHIPPED.is_file(), "no saved/policy.npz")
class Shipped(unittest.TestCase):
    def test_numpy_gives_the_trainers_answers(self) -> None:
        policy = Policy.load(SHIPPED)
        self.assertEqual(policy.inputs, 36)
        self.assertLess(policy.check(), 1e-5)


if __name__ == "__main__":
    unittest.main()
