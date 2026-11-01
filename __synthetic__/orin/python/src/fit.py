"""Fits the homography, the matrix that turns the camera's pixels into metres in the
pole's plane, from the calibration's measured points (synthetic-loop-dia.md,
Calibration). It also holds the fitted matrix for each frame after.

The metres come from MuJoCo, which knows where each point truly is, so nothing here
works out a position. The arithmetic of the matrix and the lens number is a copy of
__live__/orin/python/src/fit.py (live and synthetic share no code, AGENTS.md), whose
stops come from hand measurements instead.

The fit's points have known real positions, so they also fix the lens number: the one
that leaves the matrix closest to all of them.
"""

import json
import os
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

import cv2
import numpy as np

from lens import Optics, correct


class FitError(ValueError):
    """The fit cannot run: the points do not fix a matrix."""


@dataclass(frozen=True)
class Pair:
    point: str  # which marker in which pose, for the report
    pixel: tuple[float, float]  # (u, v): measured in the picture
    metres: tuple[float, float]  # (x, z): MuJoCo's, in the pole's plane


def _matrix(pixels: np.ndarray, metres: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """The matrix for these pixels, and each point's distance from where it lands."""
    # Method 0: least squares over all the points, with no point rejected.
    matrix, _ = cv2.findHomography(pixels, metres, method=0)
    if matrix is None:
        raise FitError("the points do not fix a matrix: check that the poses really differ")
    back = cv2.perspectiveTransform(metres.reshape(-1, 1, 2), np.linalg.inv(matrix)).reshape(-1, 2)
    return matrix, np.linalg.norm(back - pixels, axis=1)


def _lens_number(pixels: np.ndarray, metres: np.ndarray, optics: Optics, calibration: dict) -> float:
    """The lens number in the settings' range whose matrix has the smallest worst
    point. The range is searched in steps, because the worst point is not smooth."""
    low, high, step = calibration["lens_k_min"], calibration["lens_k_max"], calibration["lens_k_step"]
    numbers = low + step * np.arange(int(round((high - low) / step)) + 1)
    return float(min(numbers, key=lambda k: _matrix(correct(pixels, optics, k), metres)[1].max()))


def fit(pairs: list[Pair], calibration: dict, optics: Optics) -> dict:
    """The report of one fit, with the lens number, `H` from corrected pixels to
    metres, and each point's residual: how far its corrected pixel is from where the
    matrix puts it."""
    pixels = np.asarray([pair.pixel for pair in pairs], dtype=np.float64)
    metres = np.asarray([pair.metres for pair in pairs], dtype=np.float64)
    _, uncorrected = _matrix(pixels, metres)
    k = _lens_number(pixels, metres, optics, calibration)
    corrected = correct(pixels, optics, k)
    matrix, residuals = _matrix(corrected, metres)
    worst = float(residuals.max())
    limit = float(calibration["residual_px"])
    return {
        "ok": bool(worst <= limit),
        "matrix": matrix.tolist(),
        "lens_k": k,
        "optics": optics.to_json(),
        "residuals": [{"point": pair.point, "px": float(px)} for pair, px in zip(pairs, residuals)],
        "worst_px": worst,
        "worst_uncorrected_px": float(uncorrected.max()),
        "limit_px": limit,
        "pairs": [
            {"point": pair.point, "pixels": list(pair.pixel), "corrected": fixed.tolist(), "metres": list(pair.metres)}
            for pair, fixed in zip(pairs, corrected)
        ],
    }


def store(report: dict, frame: tuple[int, int], path: Path) -> None:
    """Writes the fit and everything it came from. Written beside the target first
    and then moved over it, so a failure midway never leaves half a matrix."""
    record = {**report, "frame": list(frame), "date": datetime.now(timezone.utc).isoformat(timespec="seconds")}
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(record, indent=2))
    os.replace(temporary, path)


@dataclass(frozen=True)
class Homography:
    """The stored matrix, held in memory by each synthetic Orin for its whole run."""

    matrix: np.ndarray
    lens_k: float
    optics: Optics

    @staticmethod
    def load(path: Path, frame: tuple[int, int]) -> "Homography":
        record = json.loads(path.read_text())
        if tuple(record["frame"]) != frame:
            raise ValueError(f"{path}: fitted on {record['frame']} frames, and these are {frame}")
        return Homography(np.asarray(record["matrix"], dtype=np.float64), record["lens_k"], Optics(**record["optics"]))

    def to_metres(self, u: float, v: float) -> tuple[float, float]:
        """A pixel's (x, z) in metres in the pole's plane: the lens's bend undone,
        then through the matrix."""
        corrected = correct(np.array([[u, v]], dtype=np.float64), self.optics, self.lens_k)
        x, z = cv2.perspectiveTransform(corrected.reshape(1, 1, 2), self.matrix).reshape(2)
        return float(x), float(z)
