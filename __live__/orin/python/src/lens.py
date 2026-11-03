"""Undoes the lens's bend, which a homography cannot. A homography maps straight
lines to straight lines, and this wide-angle lens bows them outward from the middle
of the picture: barrel distortion.

The bend is measured once, on its own, by the lens calibration (the /lens page,
`lens_calibration.py`): OpenCV's calibrateCamera on stills of a printed board, stored
in lens.json. Its numbers are the camera matrix (focal lengths fx, fy and the lens's
centre cx, cy, in pixels) and the bend's five coefficients k1, k2, p1, p2, k3 (OpenCV,
"Camera Calibration").

lens.json is in full-sensor pixels, 1456 x 1088, and the detector's frames are halved,
728 x 544. Halving every pixel position halves fx, fy, cx and cy. The coefficients act
on positions measured in focal lengths, (u - cx) / fx, which halving leaves the same,
so they stay as they are:

    fx = 1846.9 / 2 = 923.5    cx = 679.1 / 2 = 339.6   (2026-11-02's result)

The camera's own halving may also shift every pixel by up to a quarter of one,
depending on whether it lines up pixel corners or pixel centres, which NVIDIA does
not document. Most of that is taken up by the matrix itself: undoing the bend on a
grid over the whole picture, once as it is and once shifted a quarter pixel, the two
differed by up to 0.178 px, and by at most 0.043 px once a matrix was fitted between
them (2026-11-02's lens), against the fit's 1 px limit.

Undoing the bend has no formula, so OpenCV's undistortPoints finds it by repeating.
Its default stopping point left 0.07 px at the picture's corners with this lens
(checked by bending the answers back with projectPoints, 2026-11-02); `CRITERIA`
repeats until the change is under 1e-12, which left under 1e-12 px.
"""

import json
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np

# OpenCV stops when either is reached: this many repeats, or a change this small.
CRITERIA = (cv2.TERM_CRITERIA_COUNT | cv2.TERM_CRITERIA_EPS, 100, 1e-12)


class LensError(ValueError):
    """There is no usable lens calibration. The message says what to do, for whoever
    is at the rig."""


@dataclass(frozen=True)
class Lens:
    """The lens calibration, in the pixels of the frames it is used on."""

    camera_matrix: np.ndarray
    dist: np.ndarray
    # Which calibration this is: its file, its date and its average error.
    path: str
    date: str
    rms_px: float

    @staticmethod
    def load(path: Path, frame: tuple[int, int]) -> "Lens":
        """lens.json, scaled to a `frame` of (width, height) pixels."""
        try:
            record = json.loads(path.read_text())
        except FileNotFoundError:
            raise LensError(f"measure the lens first, on the /lens page: there is no {path}") from None
        except (OSError, ValueError) as error:
            raise LensError(f"{path} could not be read, so measure the lens again on the /lens page: {error}") from None
        width, height = frame
        across, down = width / record["width"], height / record["height"]
        if abs(across - down) > 1e-9:
            raise LensError(
                f"{path} is for {record['width']} x {record['height']} pixels, which does not scale to "
                f"{width} x {height}: measure the lens again"
            )
        scale = np.diag([across, across, 1.0])
        return Lens(
            scale @ np.asarray(record["camera_matrix"], dtype=np.float64),
            np.asarray(record["dist"], dtype=np.float64),
            str(path),
            record["date"],
            float(record["rms_px"]),
        )

    def undo(self, pixels: np.ndarray) -> np.ndarray:
        """Pixels `(u, v)`, one per row, where a lens with no bend but the same focal
        lengths and centre would have put them."""
        points = pixels.reshape(-1, 1, 2).astype(np.float64)
        straight = cv2.undistortPoints(points, self.camera_matrix, self.dist, R=None, P=self.camera_matrix, criteria=CRITERIA)
        return straight.reshape(-1, 2)

    def to_json(self) -> dict:
        """What the matrix was fitted with, stored beside it: every later frame must
        have its bend undone with these same numbers before the matrix is applied."""
        return {
            "camera_matrix": self.camera_matrix.tolist(),
            "dist": self.dist.tolist(),
            "path": self.path,
            "date": self.date,
            "rms_px": self.rms_px,
        }
