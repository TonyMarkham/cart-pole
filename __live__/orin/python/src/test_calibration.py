"""Checks the measurement's and the fit's arithmetic with known answers. Needs no
camera, GPU or board.

usage (from __live__/orin/python): uv run python src/test_calibration.py
"""

import json
import tempfile
import unittest
from pathlib import Path

import cv2
import numpy as np

from fit import POSE_KEYS, POSES, FitError, end_metres, fit, pose_key, stop_positions
from lens import Lens, LensError
from lens_calibration import focal_px, make_board, plan, positions, rect_size, shape, solve, turn
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
}

# A lens like the rig's: its 2026-11-02 calibration, rounded, in the detector's
# 728 x 544 frames, and the same lens with no bend.
BENT = Lens(
    np.array([[923.5, 0.0, 339.6], [0.0, 918.8, 234.2], [0.0, 0.0, 1.0]]),
    np.array([-0.519, 0.452, 0.014, 0.003, -0.417]),
    "lens.json", "2026-11-03T00:13:38Z", 0.26,
)
STRAIGHT = Lens(BENT.camera_matrix, np.zeros(5), "lens.json", "2026-11-03T00:13:38Z", 0.26)


def seen_through(matrix, metres) -> tuple[float, float]:
    """Where a camera described by `matrix`, metres to pixels, sees a point."""
    point = cv2.perspectiveTransform(np.asarray([[metres]], dtype=np.float64), matrix)
    return float(point[0, 0, 0]), float(point[0, 0, 1])


def still(u: float, v: float) -> MarkerPoint:
    return MarkerPoint(u, v, 0.0, 0, 30)


# An oblique camera: about 2.5 mm per pixel, with some perspective.
CAMERA = np.array([[400.0, 8.0, 360.0], [4.0, -400.0, 240.0], [0.02, 0.01, 1.0]])


def bent(u: float, v: float, lens: Lens) -> tuple[float, float]:
    """Where `lens` puts a point that a lens with no bend puts at (u, v): OpenCV's
    projectPoints, the bend's own formula, on the point in focal lengths."""
    (fx, _, cx), (_, fy, cy), _ = lens.camera_matrix
    ray = np.array([[(u - cx) / fx, (v - cy) / fy, 1.0]])
    seen, _ = cv2.projectPoints(ray, np.zeros(3), np.zeros(3), lens.camera_matrix, lens.dist)
    return float(seen[0, 0, 0]), float(seen[0, 0, 1])


def poses_seen(positions, lens: Lens = STRAIGHT) -> dict:
    seen = {}
    for stop, pole in POSES:
        x = getattr(positions, stop)
        end = end_metres(stop, pole, x, 0.082, 0.450)
        seen[pose_key(stop, pole)] = {
            "Pivot": still(*bent(*seen_through(CAMERA, (x, 0.082)), lens)),
            "End": still(*bent(*seen_through(CAMERA, end), lens)),
        }
    return seen


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


class Poses(unittest.TestCase):
    def test_there_are_seven_and_the_centre_has_only_the_hanging_one(self):
        self.assertEqual(len(POSE_KEYS), 7)
        self.assertEqual([pole for stop, pole in POSES if stop == "centre"], ["hanging"])

    def test_end_hangs_below_and_stands_above_pivot(self):
        self.assertEqual(end_metres("centre", "hanging", 0.1, 0.082, 0.450), (0.1, 0.082 - 0.450))
        self.assertEqual(end_metres("motor", "upright", 0.1, 0.082, 0.450), (0.1, 0.082 + 0.450))

    def test_held_horizontal_the_pole_points_away_from_the_centre(self):
        # The idler's end is on the left as the camera sees it, so its End is further left.
        self.assertEqual(end_metres("idler", "horizontal", -0.45, 0.082, 0.450), (-0.45 - 0.450, 0.082))
        self.assertEqual(end_metres("motor", "horizontal", 0.44, 0.082, 0.450), (0.44 + 0.450, 0.082))


class UndoingTheBend(unittest.TestCase):
    def test_no_bend_moves_nothing(self):
        pixels = np.array([[100.0, 50.0], [364.0, 272.0]])
        np.testing.assert_allclose(STRAIGHT.undo(pixels), pixels, atol=1e-9)

    def test_a_bent_point_goes_back_where_it_belongs_even_in_a_corner(self):
        # The picture's corners, where the bend is largest: about 60 px with this lens.
        for u, v in ((5.0, 5.0), (720.0, 538.0), (360.0, 30.0), (339.6, 234.2)):
            np.testing.assert_allclose(BENT.undo(np.array([bent(u, v, BENT)])), [[u, v]], atol=1e-6)

    def test_the_lens_file_is_halved_for_the_detectors_frames(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "lens.json"
            path.write_text(json.dumps({
                "camera_matrix": [[1846.9, 0, 679.1], [0, 1837.6, 468.4], [0, 0, 1]],
                "dist": [-0.519, 0.452, 0.014, 0.003, -0.417],
                "width": 1456, "height": 1088, "date": "2026-11-03T00:13:38Z", "rms_px": 0.26,
            }))
            lens = Lens.load(path, (728, 544))
            np.testing.assert_allclose(lens.camera_matrix, [[923.45, 0, 339.55], [0, 918.8, 234.2], [0, 0, 1]])
            np.testing.assert_allclose(lens.dist, [-0.519, 0.452, 0.014, 0.003, -0.417])
            with self.assertRaisesRegex(LensError, "does not scale"):
                Lens.load(path, (728, 600))
            with self.assertRaisesRegex(LensError, "measure the lens first"):
                Lens.load(Path(folder) / "none.json", (728, 544))


class Fit(unittest.TestCase):
    def test_a_true_camera_gives_back_its_own_points(self):
        positions = positions_of_the_example()
        report = fit(poses_seen(positions), GAPS_MM, CALIBRATION, 59_822, STRAIGHT)
        self.assertEqual(report["gaps_mm"], GAPS_MM)
        self.assertTrue(report["within_limit"])
        self.assertEqual(report["lens"]["date"], "2026-11-03T00:13:38Z")
        # OpenCV's fit stops refining at about 1e-5 px, far finer than the detector.
        self.assertLess(report["worst_px"], 1e-3)
        matrix = np.asarray(report["matrix"])
        u, v = seen_through(CAMERA, (0.1, 0.0))
        x, z = seen_through(matrix, (u, v))
        self.assertAlmostEqual(x, 0.1, places=5)
        self.assertAlmostEqual(z, 0.0, places=5)

    def test_the_lens_calibrations_bend_is_undone_before_the_fit(self):
        # Seen through the rig's lens, the points are bent by up to tens of pixels; a
        # matrix alone cannot follow that, and with the bend undone it fits exactly.
        positions = positions_of_the_example()
        report = fit(poses_seen(positions, BENT), GAPS_MM, CALIBRATION, 59_822, BENT)
        self.assertTrue(report["within_limit"])
        self.assertLess(report["worst_px"], 1e-3)
        self.assertGreater(report["worst_uncorrected_px"], 1.0)
        self.assertEqual(report["lens"]["dist"], BENT.dist.tolist())

    def test_a_bad_point_fails_the_fit(self):
        positions = positions_of_the_example()
        poses = poses_seen(positions)
        good = poses["centre hanging"]["Pivot"]
        poses["centre hanging"]["Pivot"] = still(good.u + 8.0, good.v)
        report = fit(poses, GAPS_MM, CALIBRATION, 59_822, STRAIGHT)
        self.assertFalse(report["within_limit"])
        self.assertGreater(report["worst_px"], 1.0)

    def test_every_pose_is_required(self):
        poses = poses_seen(positions_of_the_example())
        del poses["idler upright"]
        with self.assertRaisesRegex(FitError, "idler upright"):
            fit(poses, GAPS_MM, CALIBRATION, 59_822, STRAIGHT)

    def test_both_gaps_are_required(self):
        poses = poses_seen(positions_of_the_example())
        with self.assertRaisesRegex(FitError, "idler"):
            fit(poses, {"motor": 5.0}, CALIBRATION, 59_822, STRAIGHT)

    def test_a_far_off_end_point_fails_the_fit(self):
        # End held level is the furthest point from the rest, and the fit leans towards
        # it: with this camera, End 2, 4 and 8 px off there left worst points of 0.365,
        # 0.736 and 1.513 px (measured on the Orin, 2026-11-02). So only the 8 px fails.
        poses = poses_seen(positions_of_the_example())
        good = poses["idler horizontal"]["End"]
        poses["idler horizontal"]["End"] = still(good.u + 8.0, good.v)
        self.assertFalse(fit(poses, GAPS_MM, CALIBRATION, 59_822, STRAIGHT)["within_limit"])


class LensMeasurement(unittest.TestCase):
    LENS = {
        "distance_mm": 1000, "measured_marker_px": 18.5, "measured_at_mm": 2000, "box_margin": 0.2,
        "tilt_deg": 30, "tilts": ["left", "right", "top", "bottom"],
        "tilt_spots": ["centre", "top left", "top right", "bottom left", "bottom right"],
    }
    BOARD = {
        "squares_x": 6, "squares_y": 4, "square_mm": 42.0, "marker_mm": 31.5,
        "printed_square_mm": 43.0, "dictionary": "DICT_4X4_50",
    }

    def test_the_box_is_the_board_at_1_m_with_room(self):
        # 32.25 / 18.5 x 1000 / 2000 = 0.872 mm per pixel; 258 / 0.872 = 296.0 and
        # 172 / 0.872 = 197.3, then x 1.2.
        self.assertEqual(rect_size(dict(self.LENS, box_margin=0.0), self.BOARD), (296, 197))
        self.assertEqual(rect_size(self.LENS, self.BOARD), (355, 237))

    def test_the_flat_positions_come_first_then_the_tilted(self):
        # 1456 / 355 -> 5 across, 1088 / 237 -> 5 down; then 5 spots x 4 sides.
        every = plan(self.LENS, self.BOARD, 1456, 1088)
        self.assertEqual(len(every), 25 + 20)
        self.assertEqual({each.tilt for each in every[:25]}, {""})
        self.assertEqual([each.tilt for each in every[25:29]], ["left", "right", "top", "bottom"])
        centre = every[25].rect
        # The picture's centre, 728, 544, to within the half pixel a whole-pixel box allows.
        self.assertAlmostEqual(centre.x + centre.w / 2, 728, delta=0.5)
        self.assertAlmostEqual(centre.y + centre.h / 2, 544, delta=0.5)

    def test_a_flat_shape_is_the_board_in_the_middle_of_its_box(self):
        rect = plan(self.LENS, self.BOARD, 1456, 1088)[0].rect
        (left, top), (right, _), (_, bottom), _ = shape(self.LENS, self.BOARD, rect, "")
        self.assertAlmostEqual(right - left, 296.0, delta=0.2)
        self.assertAlmostEqual(bottom - top, 197.3, delta=0.2)
        self.assertAlmostEqual((left + right) / 2, rect.x + rect.w / 2, delta=0.1)

    def test_a_turned_shape_has_its_far_side_shorter(self):
        # A pinhole camera shrinks a length in proportion to its distance. Turned 30
        # degrees, the left side is 129 x sin 30 = 64.5 mm farther than the centre and
        # the right side 64.5 mm nearer: (1000 - 64.5) / (1000 + 64.5) = 935.5 / 1064.5.
        rect = plan(self.LENS, self.BOARD, 1456, 1088)[0].rect
        top_left, top_right, bottom_right, bottom_left = shape(self.LENS, self.BOARD, rect, "left")
        left, right = bottom_left[1] - top_left[1], bottom_right[1] - top_right[1]
        self.assertAlmostEqual(left / right, 935.5 / 1064.5, places=2)

    def test_the_turn_is_measured_wherever_and_however_the_board_is_held(self):
        board = make_board(self.BOARD)
        # The camera's scale measured from the board: 1000 / 0.872 = 1147 px.
        self.assertAlmostEqual(focal_px(self.LENS, self.BOARD), 1147.3, places=1)
        camera = np.array([[1147.3, 0, 728], [0, 1147.3, 544], [0, 0, 1]])
        corners = board.getChessboardCorners()
        centre = corners.mean(axis=0)
        upside_down = cv2.Rodrigues(np.array([0.0, 0.0, np.pi]))[0]
        for axis, side in ((1, "left"), (0, "bottom")):
            for degrees in (25, -35, 10):
                for distance_m in (0.8, 1.0, 1.3):
                    for held in (np.eye(3), upside_down):
                        # About the picture's up axis, a positive angle sends the left
                        # side away; about its across axis, the bottom side.
                        turned = np.zeros(3)
                        turned[axis] = np.radians(degrees)
                        rotation = cv2.Rodrigues(turned)[0] @ held
                        tvec = -rotation @ centre + np.array([0.0, 0.0, distance_m])
                        seen, _ = cv2.projectPoints(corners, cv2.Rodrigues(rotation)[0], tvec, camera, None)
                        sides = turn(board, seen.reshape(-1, 2), np.arange(len(corners)), 1147.3, 1456, 1088)
                        self.assertAlmostEqual(sides[side], degrees, delta=0.1, msg=(side, degrees, distance_m))
                        # Four corners, the least: the frame that stopped the view on
                        # 2026-11-02 had four. The four at the board's ends, so they
                        # are not all on one line.
                        few = np.array([0, 4, 10, 14])
                        sides = turn(board, seen.reshape(-1, 2)[few], few, 1147.3, 1456, 1088)
                        self.assertAlmostEqual(sides[side], degrees, delta=0.1, msg=("four", side, degrees))
        self.assertIsNone(turn(board, corners[:3, :2], np.arange(3), 1147.3, 1456, 1088))
        # Four on one row of the board say nothing of its turn; OpenCV may refuse
        # them or answer, and either way the view must carry on.
        turn(board, corners[:4, :2] * 1000, np.arange(4), 1147.3, 1456, 1088)

    def test_the_positions_reach_every_edge(self):
        tiles = positions(1456, 1088, (224, 150))
        self.assertEqual(len(tiles), 7 * 8)
        self.assertEqual((tiles[0].x, tiles[0].y), (0, 0))
        self.assertEqual((tiles[-1].x + tiles[-1].w, tiles[-1].y + tiles[-1].h), (1456, 1088))

    def test_a_known_camera_is_found_again(self):
        board = make_board(self.BOARD)
        camera = np.array([[1740.0, 0, 728], [0, 1740.0, 544], [0, 0, 1]])
        dist = np.array([-0.1, 0.05, 0, 0, 0])
        corners = board.getChessboardCorners()
        random = np.random.default_rng(0)
        views = []
        for _ in range(12):
            rvec = random.normal(0, 0.25, 3)
            tvec = np.array([random.uniform(-0.6, 0.6), random.uniform(-0.4, 0.4), 2.0])
            seen, _ = cv2.projectPoints(corners, rvec, tvec, camera, dist)
            views.append({"corners": seen.reshape(-1, 2).tolist(), "ids": list(range(len(corners)))})
        result = solve(board, views, 1456, 1088)
        self.assertLess(result["rms_px"], 1e-3)
        self.assertAlmostEqual(result["camera_matrix"][0][0], 1740.0, places=1)
        np.testing.assert_allclose(result["dist"][:2], [-0.1, 0.05], atol=1e-3)
        self.assertTrue(result["monotonic"])


if __name__ == "__main__":
    unittest.main()
