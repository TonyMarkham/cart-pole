"""Checks the measurement's, the fit's and the matrix's arithmetic with known answers.
Needs no player, GPU or orchestrator.

usage (from the repo root):
  uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/test_calibration.py
"""

import json
import tempfile
import unittest
from pathlib import Path

import cv2
import numpy as np

from fit import Homography, Pair, fit, store
from lens import Optics, correct
from measure import clipped_mean

CALIBRATION = {"residual_px": 1.0, "lens_k_min": -0.2, "lens_k_max": 1.2, "lens_k_step": 0.005}

FRAME = (728, 544)
OPTICS = Optics(364.0, 272.0, 879.0)

# An oblique camera, metres to pixels: about 2.5 mm per pixel, with some perspective.
CAMERA = np.array([[400.0, 8.0, 360.0], [4.0, -400.0, 240.0], [0.02, 0.01, 1.0]])

# Where the seven poses put Pivot and End, in metres: the two rows of the hanging pole,
# the upright End above them, and the horizontal End at each side.
PLACES = [(x, z) for x in (-0.435, 0.0025, 0.440) for z in (0.082, -0.368)]
PLACES += [(-0.435, 0.532), (0.440, 0.532), (-0.885, 0.082), (0.890, 0.082)]


def seen_through(matrix, metres) -> tuple[float, float]:
    """Where a camera described by `matrix`, metres to pixels, sees a point."""
    point = cv2.perspectiveTransform(np.asarray([[metres]], dtype=np.float64), matrix)
    return float(point[0, 0, 0]), float(point[0, 0, 1])


def bent(u: float, v: float, k: float) -> tuple[float, float]:
    """Where a lens with number `k` puts a point that a perfect lens puts at (u, v):
    the point that `correct` carries back to (u, v), found by repeating."""
    target = np.array([[u, v]])
    guess = target.copy()
    for _ in range(50):
        guess = guess + target - correct(guess, OPTICS, k)
    return float(guess[0, 0]), float(guess[0, 1])


def pairs_seen(k: float = 0.0) -> list[Pair]:
    return [Pair(f"point {i}", bent(*seen_through(CAMERA, place), k), place) for i, place in enumerate(PLACES)]


class ClippedMean(unittest.TestCase):
    def test_a_far_off_detection_is_dropped(self):
        detections = [(100.0 + 0.01 * (i % 3), 200.0) for i in range(30)] + [(120.0, 200.0)]
        point = clipped_mean(detections, 3)
        self.assertEqual((point.dropped, point.count), (1, 31))
        self.assertAlmostEqual(point.u, 100.01, places=2)

    def test_identical_detections_keep_every_one(self):
        point = clipped_mean([(5.0, 6.0)] * 30, 3)
        self.assertEqual((point.u, point.v, point.spread, point.dropped), (5.0, 6.0, 0.0, 0))


class Lens(unittest.TestCase):
    def test_no_bend_moves_nothing(self):
        pixels = np.array([[100.0, 50.0], [364.0, 272.0]])
        np.testing.assert_allclose(correct(pixels, OPTICS, 0.0), pixels)

    def test_the_focal_length_comes_from_the_settings(self):
        # 6 mm over a pixel of 4.968 mm / 728 px, the loop scene's camera.
        optics = Optics.from_settings({"lens_focal_mm": 6.0, "sensor_width_mm": 4.968}, FRAME)
        self.assertAlmostEqual(optics.focal_px, 6.0 / (4.968 / 728))
        self.assertEqual((optics.cx, optics.cy), (364.0, 272.0))


class Fit(unittest.TestCase):
    def test_a_true_camera_with_no_lens_gives_back_its_own_points(self):
        report = fit(pairs_seen(), CALIBRATION, OPTICS)
        self.assertTrue(report["ok"])
        self.assertAlmostEqual(report["lens_k"], 0.0, delta=0.005)
        # OpenCV's fit stops refining at about 1e-5 px, far finer than the detector.
        self.assertLess(report["worst_px"], 1e-3)

    def test_it_finds_a_lens_it_has_not_been_told(self):
        report = fit(pairs_seen(0.44), CALIBRATION, OPTICS)
        self.assertAlmostEqual(report["lens_k"], 0.44, delta=0.01)
        self.assertLess(report["worst_px"], 0.1)
        self.assertGreater(report["worst_uncorrected_px"], 0.5)

    def test_a_bad_point_fails_the_fit(self):
        pairs = pairs_seen()
        good = pairs[0]
        pairs[0] = Pair(good.point, (good.pixel[0] + 8.0, good.pixel[1]), good.metres)
        report = fit(pairs, CALIBRATION, OPTICS)
        self.assertFalse(report["ok"])
        self.assertGreater(report["worst_px"], 1.0)


class StoredMatrix(unittest.TestCase):
    def stored(self, k: float) -> Homography:
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "homography.json"
            store(fit(pairs_seen(k), CALIBRATION, OPTICS), FRAME, path)
            self.assertEqual(json.loads(path.read_text())["frame"], list(FRAME))
            return Homography.load(path, FRAME)

    def test_a_point_it_was_not_fitted_to_comes_back_in_metres(self):
        # 45 degrees up from the right stop: inside none of the fitted poses.
        place = (0.440 + 0.318, 0.082 + 0.318)
        for k in (0.0, 0.44):
            x, z = self.stored(k).to_metres(*bent(*seen_through(CAMERA, place), k))
            self.assertAlmostEqual(x, place[0], delta=0.001)
            self.assertAlmostEqual(z, place[1], delta=0.001)

    def test_a_matrix_for_other_frames_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "homography.json"
            store(fit(pairs_seen(), CALIBRATION, OPTICS), FRAME, path)
            with self.assertRaisesRegex(ValueError, "fitted on"):
                Homography.load(path, (1456, 1088))


if __name__ == "__main__":
    unittest.main()
