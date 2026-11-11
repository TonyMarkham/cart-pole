"""Fits the homography, the matrix that turns the camera's pixels into metres in the
pole's plane, from the seven poses' measured points
(__documentation__/calibrate-w-homography.md, The calibration sequence). The poses are the synthetic
loop's (__documentation__/synthetic-loop-dia.md, The seven poses): the pole hanging at each of the three
stops, and at each end also held horizontal, pointing away from the centre, and
straight up. The hanging poses alone cover only the rail and the hanging pole, while a
swinging End reaches the pole's length past them on either side and above.

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

Each pose gives one Pivot point and one End point, 14 in all, less those of the poses
`fit_leave_out` names (the hanging three alone since 2026-11-09). Pivot is always at its
stop's x and at z_P. End, with L the pole's length:

    hanging     (x_stop, z_P - L)
    upright     (x_stop, z_P + L)
    horizontal  (x_idler - L, z_P) at the idler's end, (x_motor + L, z_P) at the motor's

The lens bends the picture, which a homography cannot follow, so every point's bend
is undone first with the lens calibration, lens.json (`lens.py`), and the matrix is
fitted to the straightened points. The 14 points then give the matrix's 8 numbers
from 28 measurements, so 20 are left to check the fit; the three hanging poses alone
give 6 points, 12 measurements, so 4 are left. Until 2026-11-02 one number
for the bend was found from these same points instead; the lens calibration measures
it from 45 stills of a board, with nothing from the rig.
"""

import json
import os
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

import cv2
import numpy as np

from lens import Lens
from measure import MarkerPoint

STOPS = ("motor", "idler", "centre")

# Each pose as (stop, pole), in the order a person meets them: an end's three poses
# while the cart rests there, then the centre's one.
POSES = (
    ("idler", "hanging"),
    ("idler", "horizontal"),
    ("idler", "upright"),
    ("motor", "hanging"),
    ("motor", "horizontal"),
    ("motor", "upright"),
    ("centre", "hanging"),
)


def pose_key(stop: str, pole: str) -> str:
    """A pose's name in the routes, the kept file and the report: "idler upright"."""
    return f"{stop} {pole}"


POSE_KEYS = tuple(pose_key(stop, pole) for stop, pole in POSES)


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


def end_metres(stop: str, pole: str, x: float, pivot_z: float, length: float) -> tuple[float, float]:
    """Where End is in a pose, from its stop's x. Held horizontal, the pole points away
    from the centre: left at the idler's end, right at the motor's."""
    if pole == "hanging":
        return x, pivot_z - length
    if pole == "upright":
        return x, pivot_z + length
    return (x - length if stop == "idler" else x + length), pivot_z


def _matrix(pixels: np.ndarray, metres: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """The matrix for these pixels, and each point's distance from where it lands."""
    # Method 0: least squares over all the points, with no point rejected.
    matrix, _ = cv2.findHomography(pixels, metres, method=0)
    if matrix is None:
        raise FitError("the points do not fix a matrix: check that the cart really moved between the stops")
    back = cv2.perspectiveTransform(metres.reshape(-1, 1, 2), np.linalg.inv(matrix)).reshape(-1, 2)
    return matrix, np.linalg.norm(back - pixels, axis=1)


def fit(
    poses: dict[str, dict[str, MarkerPoint]],
    gaps_mm: dict[str, float],
    calibration: dict,
    span_pulses: int,
    lens: Lens,
) -> dict:
    """The report of one fit: the lens calibration used, `H` from straightened pixels
    to metres, and each point's residual: how far its straightened pixel is from
    where the matrix puts it. `poses` holds each pose's points, by `pose_key`, and
    `gaps_mm` the hand measurement at the motor's end and at the idler's end.

    The poses named in the calibration's `fit_leave_out` are not fitted, and need not
    be measured: on 2026-11-09 the poses held by hand, horizontal and upright, were
    the fit's worst points by far (user: "the horizontal and the up points are
    unreliable")."""
    left_out = list(calibration["fit_leave_out"])
    unknown = [key for key in left_out if key not in POSE_KEYS]
    if unknown:
        raise FitError(f"fit_leave_out names no such pose: {', '.join(unknown)}; the poses are {', '.join(POSE_KEYS)}")
    fitted = [(stop, pole) for stop, pole in POSES if pose_key(stop, pole) not in left_out]
    # A matrix has 8 numbers and each pose gives 4 measurements, so at least 2 poses,
    # and they must not all lie at one stop.
    if len(fitted) < 2 or len({stop for stop, _ in fitted}) < 2:
        raise FitError("fit_leave_out leaves too few poses: keep at least two stops' poses")
    absent = [pose_key(stop, pole) for stop, pole in fitted if pose_key(stop, pole) not in poses]
    if absent:
        raise FitError(f"measure these poses first: {', '.join(absent)}")
    absent = [end for end in ("motor", "idler") if end not in gaps_mm]
    if absent:
        raise FitError(f"measure the gap at these ends first: {', '.join(absent)}")
    positions = stop_positions(calibration, gaps_mm["idler"] / 1_000, gaps_mm["motor"] / 1_000, span_pulses)
    pivot_z = calibration["pivot_z"]
    length = calibration["pole_length"]
    names, pixels, metres = [], [], []
    for stop, pole in fitted:
        key = pose_key(stop, pole)
        x = getattr(positions, stop)
        for marker, metre in (("Pivot", (x, pivot_z)), ("End", end_metres(stop, pole, x, pivot_z, length))):
            point = poses[key][marker]
            names.append(f"{marker} {key}")
            pixels.append((point.u, point.v))
            metres.append(metre)
    pixels_array = np.asarray(pixels, dtype=np.float64)
    metres_array = np.asarray(metres, dtype=np.float64)
    # The same fit with the bend left in, reported only, to show what undoing it did.
    _, uncorrected = _matrix(pixels_array, metres_array)
    corrected = lens.undo(pixels_array)
    matrix, residuals = _matrix(corrected, metres_array)
    worst = float(residuals.max())
    limit = float(calibration["residual_px"])
    return {
        # Over the limit is not a failure: the robot balancing decides whether the
        # matrix is good enough, and a person may accept it (user, 2026-11-02).
        "within_limit": bool(worst <= limit),
        "accepted": False,
        "matrix": matrix.tolist(),
        "left_out": left_out,
        "lens": lens.to_json(),
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


def store(report: dict, poses: dict[str, dict[str, MarkerPoint]], frame: tuple[int, int], path: Path) -> None:
    """Writes the fit and everything it came from. Written beside the target first
    and then moved over it, so a failure midway never leaves half a matrix."""
    record = {
        **report,
        "frame": list(frame),
        "date": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "poses": {key: {name: asdict(point) for name, point in points.items()} for key, points in poses.items()},
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(record, indent=2))
    os.replace(temporary, path)
