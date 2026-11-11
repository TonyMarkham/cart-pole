"""Turns one stop's many detections of a still marker into one point, trusting no
single detection (__documentation__/calibrate-w-homography.md, The measurement at a stop).

The detector can return a wrong box now and then, and one detection 20 px off
would pull a plain mean of 30 by 0.7 px, more than the detector's own error. So
this takes a clipped mean: find the middle of the detections with their median,
which a far-off point cannot drag, drop every detection too far from it, and take
the plain mean of the rest.
"""

from dataclasses import asdict, dataclass

import numpy as np


@dataclass(frozen=True)
class MarkerPoint:
    u: float  # pixels across the frame, 0 at the left
    v: float  # pixels down the frame, 0 at the top
    spread: float  # standard deviation about the median: root mean square distance, pixels
    dropped: int  # detections clipped away
    count: int  # detections there were before clipping

    def to_json(self) -> dict:
        return asdict(self)


def clipped_mean(detections: list[tuple[float, float]], clip_k: float) -> MarkerPoint:
    """One point from detections `(u, v)`. A detection is dropped when it is more
    than `clip_k` spreads from the median."""
    points = np.asarray(detections, dtype=np.float64)
    centre = np.median(points, axis=0)
    distances = np.linalg.norm(points - centre, axis=1)
    # About the median, not about the mean distance: `distances.std()` would measure
    # only how much the distances vary, and clip ordinary scatter (seen on the rig).
    spread = float(np.sqrt(np.mean(distances**2)))
    kept = points[distances <= clip_k * spread] if spread > 0 else points
    u, v = kept.mean(axis=0)
    return MarkerPoint(float(u), float(v), spread, len(points) - len(kept), len(points))
