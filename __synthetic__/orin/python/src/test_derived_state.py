"""Checks the five numbers with answers that can be worked by hand. Needs no player, GPU
or orchestrator.

usage (from the repo root):
  uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/test_derived_state.py
"""

import math
import unittest

from derived_state import NOT_DETECTED, derive

CENTRE_X = 0.0025
LENGTH = 0.45  # Pivot to End, m
PIVOT_Z = 0.082  # m
MISSING = [*NOT_DETECTED, *NOT_DETECTED]


def row(age_ms: float, pivot_x: float, theta: float) -> list[float]:
    """A pool row with the cart's Pivot at `pivot_x` and the pole at `theta` from up."""
    return [age_ms, pivot_x, PIVOT_Z, pivot_x + LENGTH * math.sin(theta), PIVOT_Z + LENGTH * math.cos(theta)]


def padded(*rows: list[float]) -> list[list[float]]:
    """Six rows: the given ones, then entries of no detection, 40 ms apart."""
    extra = [[rows[-1][0] - 40.0 * (i + 1), *MISSING] for i in range(6 - len(rows))]
    return [*rows, *extra]


class WhereThingsAre(unittest.TestCase):
    def test_hanging_at_the_centre_stop(self):
        state = derive(padded(row(0.0, CENTRE_X, math.pi)), CENTRE_X)
        self.assertAlmostEqual(state.cart, 0.0)
        self.assertAlmostEqual(state.cos_theta, -1.0)
        self.assertAlmostEqual(state.sin_theta, 0.0)

    def test_upright_with_the_cart_out_to_the_left(self):
        state = derive(padded(row(0.0, CENTRE_X - 0.3, 0.0)), CENTRE_X)
        self.assertAlmostEqual(state.cart, -0.3)
        self.assertAlmostEqual(state.cos_theta, 1.0)
        self.assertAlmostEqual(state.sin_theta, 0.0)

    def test_leaning_towards_plus_x(self):
        # End 0.27 along and 0.36 above Pivot: a 3-4-5 triangle of length 0.45.
        newest = [0.0, 0.1, PIVOT_Z, 0.1 + 0.27, PIVOT_Z + 0.36]
        state = derive(padded(newest), CENTRE_X)
        self.assertAlmostEqual(state.cos_theta, 0.8)
        self.assertAlmostEqual(state.sin_theta, 0.6)


class Speeds(unittest.TestCase):
    def test_the_cart_moving_towards_plus_x(self):
        # 8 mm in 40 ms is 0.2 m/s.
        state = derive(padded(row(0.0, 0.108, 0.0), row(-40.0, 0.100, 0.0)), CENTRE_X)
        self.assertAlmostEqual(state.cart_speed, 0.2)

    def test_the_pole_turning_from_up_towards_plus_x(self):
        # 0.2 rad in 50 ms is 4 rad/s.
        state = derive(padded(row(0.0, 0.0, 0.5), row(-50.0, 0.0, 0.3)), CENTRE_X)
        self.assertAlmostEqual(state.pole_speed, 4.0)

    def test_the_gap_is_the_true_time_between_the_entries(self):
        # The same 8 mm, over 34 ms instead of 40: the gaps between frames vary.
        state = derive(padded(row(0.0, 0.108, 0.0), row(-34.0, 0.100, 0.0)), CENTRE_X)
        self.assertAlmostEqual(state.cart_speed, 0.008 / 0.034)

    def test_swinging_through_hanging_is_a_small_turn(self):
        # From 0.1 rad short of pi on one side to 0.1 rad short of it on the other: theta
        # jumps from about +3.04 to -3.04, but the pole has turned only 0.2 rad.
        forward = padded(row(0.0, 0.0, -math.pi + 0.1), row(-50.0, 0.0, math.pi - 0.1))
        backward = padded(row(0.0, 0.0, math.pi - 0.1), row(-50.0, 0.0, -math.pi + 0.1))
        self.assertAlmostEqual(derive(forward, CENTRE_X).pole_speed, 4.0)
        self.assertAlmostEqual(derive(backward, CENTRE_X).pole_speed, -4.0)


class MissingMarkers(unittest.TestCase):
    def test_a_first_frame_has_positions_but_no_speeds(self):
        state = derive(padded(row(0.0, 0.1, 0.0)), CENTRE_X)
        self.assertIsNotNone(state.cart)
        self.assertIsNone(state.cart_speed)
        self.assertIsNone(state.pole_speed)

    def test_no_pivot_leaves_everything_unknown(self):
        newest = [0.0, *NOT_DETECTED, 0.1, -0.3]
        state = derive(padded(newest, row(-40.0, 0.1, 0.0)), CENTRE_X)
        self.assertEqual(state.to_json(), dict.fromkeys(state.to_json()))

    def test_no_end_leaves_the_cart_known_and_the_pole_unknown(self):
        newest = [0.0, 0.108, PIVOT_Z, *NOT_DETECTED]
        state = derive(padded(newest, row(-40.0, 0.100, 0.0)), CENTRE_X)
        self.assertAlmostEqual(state.cart, 0.108 - CENTRE_X)
        self.assertAlmostEqual(state.cart_speed, 0.2)
        self.assertIsNone(state.cos_theta)
        self.assertIsNone(state.sin_theta)
        self.assertIsNone(state.pole_speed)

    def test_a_speed_reaches_back_past_a_missed_frame(self):
        # The middle frame lost End, so the turn is measured across both gaps. The
        # cart's speed still uses that frame's Pivot.
        middle = [-40.0, 0.104, PIVOT_Z, *NOT_DETECTED]
        state = derive(padded(row(0.0, 0.108, 0.5), middle, row(-80.0, 0.100, 0.3)), CENTRE_X)
        self.assertAlmostEqual(state.cart_speed, 0.1)
        self.assertAlmostEqual(state.pole_speed, 0.2 / 0.08)


if __name__ == "__main__":
    unittest.main()
