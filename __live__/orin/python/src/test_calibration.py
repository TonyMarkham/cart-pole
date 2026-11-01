"""Checks the measurement's and the fit's arithmetic with known answers. Needs no
camera, GPU or board.

usage (from __live__/orin/python): uv run python src/test_calibration.py
"""

import unittest

import cv2
import numpy as np

from fit import FitError, fit, stop_positions
from lens import Optics, correct
from measure import MarkerPoint, clipped_mean

# The plan's worked example: 5 mm at the left end, the idler's, 7 mm at the right.
GAPS_MM = {"idler": 5.0, "motor": 7.0}
GAP_IDLER_M, GAP_MOTOR_M = 0.005, 0.007

CALIBRATION = {
    "left_wall_x": -0.485,
    "right_wall_x": 0.470,
    "left_face_to_pivot": 0.035,
    "right_face_to_pivot": 0.035,
    "pivot_z": 0.082,
    "pole_length": 0.450,
    "metres_per_pulse": 0.000015,
    "residual_px": 1.0,
    "lens_k_min": -0.2,
    "lens_k_max": 1.2,
    "lens_k_step": 0.005,
}

OPTICS = Optics(364.0, 272.0, 870.0)


def seen_through(matrix, metres) -> tuple[float, float]:
    """Where a camera described by `matrix`, metres to pixels, sees a point."""
    point = cv2.perspectiveTransform(np.asarray([[metres]], dtype=np.float64), matrix)
    return float(point[0, 0, 0]), float(point[0, 0, 1])


def still(u: float, v: float) -> MarkerPoint:
    return MarkerPoint(u, v, 0.0, 0, 30)


# An oblique camera: about 2.5 mm per pixel, with some perspective.
CAMERA = np.array([[400.0, 8.0, 360.0], [4.0, -400.0, 240.0], [0.02, 0.01, 1.0]])


def bent(u: float, v: float, k: float) -> tuple[float, float]:
    """Where a lens with number `k` puts a point that a perfect lens puts at (u, v):
    the point that `correct` carries back to (u, v), found by repeating."""
    target = np.array([[u, v]])
    guess = target.copy()
    for _ in range(50):
        guess = guess + target - correct(guess, OPTICS, k)
    return float(guess[0, 0]), float(guess[0, 1])


def stops_seen(positions, k: float = 0.0) -> dict:
    return {
        stop: {
            "Pivot": still(*bent(*seen_through(CAMERA, (getattr(positions, stop), 0.082)), k)),
            "End": still(*bent(*seen_through(CAMERA, (getattr(positions, stop), -0.368)), k)),
        }
        for stop in ("motor", "idler", "centre")
    }


def positions_of_the_example():
    return stop_positions(CALIBRATION, GAP_IDLER_M, GAP_MOTOR_M, 59_822)


class ClippedMean(unittest.TestCase):
    def test_a_far_off_detection_is_dropped(self):
        detections = [(100.0 + 0.01 * (i % 3), 200.0) for i in range(30)] + [(120.0, 200.0)]
        point = clipped_mean(detections, 3)
        self.assertEqual(point.dropped, 1)
        self.assertEqual(point.count, 31)
        self.assertAlmostEqual(point.u, 100.01, places=2)

    def test_ordinary_scatter_keeps_nearly_every_one(self):
        # A still marker's real detections: a small round scatter, with no wrong box.
        random = np.random.default_rng(1)
        detections = [tuple(p) for p in 100.0 + random.normal(0.0, 0.01, (1000, 2))]
        point = clipped_mean(detections, 3)
        self.assertLess(point.dropped, 10)

    def test_identical_detections_keep_every_one(self):
        point = clipped_mean([(5.0, 6.0)] * 30, 3)
        self.assertEqual((point.u, point.v, point.spread, point.dropped), (5.0, 6.0, 0.0, 0))


class Stops(unittest.TestCase):
    def test_the_span_sets_the_length_and_the_gaps_the_centre(self):
        positions = positions_of_the_example()
        # The plan's own worked example: the CAD and these gaps give -0.445 and
        # 0.428, so a centre of -0.0085 and a length of 0.873.
        self.assertAlmostEqual(positions.centre, -0.0085)
        self.assertAlmostEqual(positions.cad_length_m, 0.873)
        self.assertAlmostEqual(positions.span_m, 0.89733)
        self.assertAlmostEqual(positions.difference_m, 0.02433)

    def test_the_motors_end_is_on_the_right(self):
        positions = positions_of_the_example()
        self.assertAlmostEqual(positions.motor, -0.0085 + 0.89733 / 2)
        self.assertAlmostEqual(positions.idler, -0.0085 - 0.89733 / 2)

    def test_each_gap_belongs_to_its_own_end(self):
        wider_motor_gap = stop_positions(CALIBRATION, 0.005, 0.015, 59_822)
        # A wider gap at the motor's end, on the right, pulls the centre left.
        self.assertLess(wider_motor_gap.centre, stop_positions(CALIBRATION, 0.005, 0.005, 59_822).centre)


class Lens(unittest.TestCase):
    def test_no_bend_moves_nothing(self):
        pixels = np.array([[100.0, 50.0], [364.0, 272.0]])
        np.testing.assert_allclose(correct(pixels, OPTICS, 0.0), pixels)

    def test_the_centre_never_moves(self):
        np.testing.assert_allclose(correct(np.array([[364.0, 272.0]]), OPTICS, 0.44), [[364.0, 272.0]])

    def test_a_positive_number_moves_points_outward(self):
        fixed = correct(np.array([[600.0, 272.0]]), OPTICS, 0.44)
        self.assertGreater(fixed[0, 0], 600.0)

    def test_the_rigs_focal_length_comes_from_the_settings(self):
        settings = {
            "camera": {"width": 1456},
            "stream": {"width": 728, "height": 544},
            "calibration": {"lens_focal_mm": 6.0, "sensor_pixel_um": 3.45},
        }
        optics = Optics.from_settings(settings)
        self.assertAlmostEqual(optics.focal_px, 6.0 / 0.0069)
        self.assertEqual((optics.cx, optics.cy), (364.0, 272.0))


class Fit(unittest.TestCase):
    def test_a_true_camera_gives_back_its_own_points(self):
        positions = positions_of_the_example()
        report = fit(stops_seen(positions), GAPS_MM, CALIBRATION, 59_822, OPTICS)
        self.assertEqual(report["gaps_mm"], GAPS_MM)
        self.assertTrue(report["ok"])
        self.assertAlmostEqual(report["lens_k"], 0.0, delta=0.005)
        # OpenCV's fit stops refining at about 1e-5 px, far finer than the detector.
        self.assertLess(report["worst_px"], 1e-3)
        matrix = np.asarray(report["matrix"])
        u, v = seen_through(CAMERA, (0.1, 0.0))
        x, z = seen_through(matrix, (u, v))
        self.assertAlmostEqual(x, 0.1, places=5)
        self.assertAlmostEqual(z, 0.0, places=5)

    def test_it_finds_a_lens_it_has_not_been_told(self):
        positions = positions_of_the_example()
        report = fit(stops_seen(positions, 0.44), GAPS_MM, CALIBRATION, 59_822, OPTICS)
        self.assertAlmostEqual(report["lens_k"], 0.44, delta=0.02)
        self.assertTrue(report["ok"])
        self.assertLess(report["worst_px"], 0.1)
        self.assertGreater(report["worst_uncorrected_px"], 0.5)

    def test_a_bad_point_fails_the_fit(self):
        positions = positions_of_the_example()
        stops = stops_seen(positions)
        good = stops["centre"]["Pivot"]
        stops["centre"]["Pivot"] = still(good.u + 8.0, good.v)
        report = fit(stops, GAPS_MM, CALIBRATION, 59_822, OPTICS)
        self.assertFalse(report["ok"])
        self.assertGreater(report["worst_px"], 1.0)

    def test_every_stop_is_required(self):
        stops = stops_seen(positions_of_the_example())
        del stops["centre"]
        with self.assertRaisesRegex(FitError, "centre"):
            fit(stops, GAPS_MM, CALIBRATION, 59_822, OPTICS)

    def test_both_gaps_are_required(self):
        stops = stops_seen(positions_of_the_example())
        with self.assertRaisesRegex(FitError, "idler"):
            fit(stops, {"motor": 5.0}, CALIBRATION, 59_822, OPTICS)


if __name__ == "__main__":
    unittest.main()
