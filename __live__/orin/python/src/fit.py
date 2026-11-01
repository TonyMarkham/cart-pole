"""Fits the homography, the matrix that turns the camera's pixels into metres in the
pole's plane, from the three stops' measured points
(calibrate-w-homography.md, The calibration sequence).

Real measurements win over the CAD. The two hand-measured gaps, typed into the
page at each switch, correct where the stops sit against the CAD's walls, which
gives the rail's centre. The board's measured span sets how far apart the stops
are. Any difference between the CAD's length and the measured one is reported with
the fit, and never affects it.

The cart's left and right are as the camera sees them. The idler's end is on the
left and the motor's on the right (measured on the rig, 2026-11-01), and x grows to
the right. With the centre at x_centre and half the measured span either side:

    x_left_cad  = left_wall_x  + gap_idler + left_face_to_pivot
    x_right_cad = right_wall_x - gap_motor - right_face_to_pivot
    x_centre    = (x_left_cad + x_right_cad) / 2
    x_idler     = x_centre - span_pulses x metres_per_pulse / 2
    x_motor     = x_centre + span_pulses x metres_per_pulse / 2

The lens bends the picture, which a homography cannot follow (`lens.py`). The six
points have known real positions: three columns the measured span apart and two rows
the pole's length apart. So they also fix the lens number: the one that leaves the
matrix closest to all six points. They then give 9 numbers (the matrix's 8 and this
one) from 12 measurements, so 3 are left to check the fit.
"""

import json
import os
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

import cv2
import numpy as np

from lens import Optics, correct
from measure import MarkerPoint

STOPS = ("motor", "idler", "centre")


class FitError(ValueError):
    """The fit cannot run yet. The message says why, for whoever is at the rig."""


@dataclass(frozen=True)
class StopPositions:
    motor: float
    idler: float
    centre: float
    span_m: float
    cad_length_m: float

    @property
    def difference_m(self) -> float:
        """How much longer the measured travel is than the CAD and the gaps say."""
        return self.span_m - self.cad_length_m


def stop_positions(calibration: dict, gap_idler_m: float, gap_motor_m: float, span_pulses: int) -> StopPositions:
    """The stops' x in metres."""
    x_left_cad = calibration["left_wall_x"] + gap_idler_m + calibration["left_face_to_pivot"]
    x_right_cad = calibration["right_wall_x"] - gap_motor_m - calibration["right_face_to_pivot"]
    x_centre = (x_left_cad + x_right_cad) / 2
    span_m = span_pulses * calibration["metres_per_pulse"]
    return StopPositions(x_centre + span_m / 2, x_centre - span_m / 2, x_centre, span_m, x_right_cad - x_left_cad)


def _matrix(pixels: np.ndarray, metres: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """The matrix for these pixels, and each point's distance from where it lands."""
    # Method 0: least squares over all six points, with no point rejected.
    matrix, _ = cv2.findHomography(pixels, metres, method=0)
    if matrix is None:
        raise FitError("the six points do not fix a matrix: check that the cart really moved between the stops")
    back = cv2.perspectiveTransform(metres.reshape(-1, 1, 2), np.linalg.inv(matrix)).reshape(-1, 2)
    return matrix, np.linalg.norm(back - pixels, axis=1)


def _lens_number(pixels: np.ndarray, metres: np.ndarray, optics: Optics, calibration: dict) -> float:
    """The lens number in the settings' range whose matrix has the smallest worst
    point. The range is searched in steps, because the worst point is not smooth."""
    low, high, step = calibration["lens_k_min"], calibration["lens_k_max"], calibration["lens_k_step"]
    numbers = low + step * np.arange(int(round((high - low) / step)) + 1)
    return float(min(numbers, key=lambda k: _matrix(correct(pixels, optics, k), metres)[1].max()))


def fit(
    stops: dict[str, dict[str, MarkerPoint]],
    gaps_mm: dict[str, float],
    calibration: dict,
    span_pulses: int,
    optics: Optics,
) -> dict:
    """The report of one fit, with the lens number, `H` from corrected pixels to
    metres, and each point's residual: how far its corrected pixel is from where the
    matrix puts it. `gaps_mm` holds the hand measurement at the motor's end and at
    the idler's end."""
    absent = [stop for stop in STOPS if stop not in stops]
    if absent:
        raise FitError(f"measure these stops first: {', '.join(absent)}")
    absent = [end for end in ("motor", "idler") if end not in gaps_mm]
    if absent:
        raise FitError(f"measure the gap at these ends first: {', '.join(absent)}")
    positions = stop_positions(calibration, gaps_mm["idler"] / 1_000, gaps_mm["motor"] / 1_000, span_pulses)
    pivot_z = calibration["pivot_z"]
    end_z = pivot_z - calibration["pole_length"]
    names, pixels, metres = [], [], []
    for stop in STOPS:
        x = getattr(positions, stop)
        for marker, z in (("Pivot", pivot_z), ("End", end_z)):
            point = stops[stop][marker]
            names.append(f"{marker} {stop}")
            pixels.append((point.u, point.v))
            metres.append((x, z))
    pixels_array = np.asarray(pixels, dtype=np.float64)
    metres_array = np.asarray(metres, dtype=np.float64)
    _, uncorrected = _matrix(pixels_array, metres_array)
    k = _lens_number(pixels_array, metres_array, optics, calibration)
    corrected = correct(pixels_array, optics, k)
    matrix, residuals = _matrix(corrected, metres_array)
    worst = float(residuals.max())
    limit = float(calibration["residual_px"])
    return {
        "ok": bool(worst <= limit),
        "matrix": matrix.tolist(),
        "lens_k": k,
        "optics": optics.to_json(),
        "residuals": [{"point": name, "px": float(px)} for name, px in zip(names, residuals)],
        "worst_px": worst,
        "worst_uncorrected_px": float(uncorrected.max()),
        "limit_px": limit,
        "positions": {stop: getattr(positions, stop) for stop in STOPS},
        "gaps_mm": gaps_mm,
        "span_pulses": span_pulses,
        "span_m": positions.span_m,
        "cad_length_m": positions.cad_length_m,
        "difference_m": positions.difference_m,
        "pairs": [
            {"point": name, "pixels": list(pixel), "corrected": fixed.tolist(), "metres": list(metre)}
            for name, pixel, fixed, metre in zip(names, pixels, corrected, metres)
        ],
    }


def store(report: dict, stops: dict[str, dict[str, MarkerPoint]], frame: tuple[int, int], path: Path) -> None:
    """Writes the fit and everything it came from. Written beside the target first
    and then moved over it, so a failure midway never leaves half a matrix."""
    record = {
        **report,
        "frame": list(frame),
        "date": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "stops": {stop: {name: asdict(point) for name, point in points.items()} for stop, points in stops.items()},
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(record, indent=2))
    os.replace(temporary, path)
