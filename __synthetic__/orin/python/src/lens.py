"""Undoes a lens's bend, which a homography cannot. A homography maps straight lines to
straight lines, and a wide-angle lens bows them outward from the middle of the picture:
barrel distortion.

One number describes the bend. Each point moves along the line through the picture's
centre, further the further out it is:

    r2         = (x^2 + y^2), x and y being the point's distance from the centre, in focal lengths
    corrected  = centre + (point - centre) x (1 + k x r2)

k is positive for barrel distortion. `fit.py` finds it for each calibration, because
the measured points have known real positions. The simulated camera has no lens
distortion (the loop's scene sets none), so k should come out near 0: a k far from 0
says something else is wrong.

A copy of __live__/orin/python/src/lens.py (live and synthetic share no code, AGENTS.md),
but for how the focal length is found.
"""

from dataclasses import asdict, dataclass

import numpy as np


@dataclass(frozen=True)
class Optics:
    """The camera's fixed facts, which put k in units that do not depend on the
    frame's size."""

    cx: float  # the picture's centre, pixels across
    cy: float  # the picture's centre, pixels down
    focal_px: float  # the lens's focal length, in pixels of the detector's frames

    @staticmethod
    def from_settings(calibration: dict, frame_size: tuple[int, int]) -> "Optics":
        width, height = frame_size
        pixel_mm = calibration["sensor_width_mm"] / width
        return Optics(width / 2, height / 2, calibration["lens_focal_mm"] / pixel_mm)

    def to_json(self) -> dict:
        return asdict(self)


def correct(pixels: np.ndarray, optics: Optics, k: float) -> np.ndarray:
    """Pixels `(u, v)`, one per row, with the lens's bend undone."""
    x = (pixels[:, 0] - optics.cx) / optics.focal_px
    y = (pixels[:, 1] - optics.cy) / optics.focal_px
    grow = 1 + k * (x * x + y * y)
    return np.stack((optics.cx + x * grow * optics.focal_px, optics.cy + y * grow * optics.focal_px), axis=1)
