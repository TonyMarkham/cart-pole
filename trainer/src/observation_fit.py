"""Fits what the trainer sees to the truth, from one run's truth log.

The trainer never sees MuJoCo's truth, only the markers (synthetic-loop-dia.md,
Truth log), so its end-crash bounds and its reward have to be stated in
marker terms. This turns a run's states.csv - the true cart and pole beside
the pool's newest markers, per observation - into the two maps those
decisions need:

- cart (m) from the Pivot marker's x: a straight-line fit, and the Pivot x
  at the slide's ends (+-0.45 m) and at the end-crash margin inside them
  (0.68 m/s x 16 ms = 11 mm, synthetic-loop-dia.md, Numbers);
- pole (rad) from the Pivot-to-End direction in the image: the angle of
  (end - pivot) in pixels (x right, y down; 0 = End straight below Pivot,
  the hanging pole), fitted as pole = k x angle + c.

Markers are normalized to the 728 x 544 frame (Contracts), so differences
are scaled back to pixels before the angle is taken. Rows where Pivot or End
was not detected (-1, -1), or with no observation (--no-render), are left
out and counted.

usage (from the repo root):
  uv run --project trainer python trainer/src/observation_fit.py [states.csv]
  (default: target/trainer-run/states.csv)
"""

import csv
import math
import sys
from pathlib import Path

import numpy as np

from pool import FRAME_HEIGHT as HEIGHT
from pool import FRAME_WIDTH as WIDTH
SLIDE_END_M = 0.45  # the MJCF slide's range, +-m
MARGIN_M = 0.68 * 0.016  # the cart's travel in the longest gap at top speed

REPO = Path(__file__).resolve().parents[2]


def fit(x: np.ndarray, y: np.ndarray, degree: int = 1) -> tuple[np.ndarray, np.ndarray]:
    """Least-squares polynomial y ~ x; returns the coefficients (highest
    power first) and the residuals."""
    coefficients = np.polyfit(x, y, degree)
    return coefficients, y - np.polyval(coefficients, x)


def main() -> int:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else REPO / "target" / "trainer-run" / "states.csv"
    with open(path, newline="") as f:
        rows = list(csv.DictReader(f))
    usable = [
        r for r in rows
        if r["pivot_x"] and r["end_x"] and float(r["pivot_x"]) != -1 and float(r["end_x"]) != -1
    ]
    print(f"{path}: {len(rows)} rows, {len(usable)} with Pivot and End detected")
    if len(usable) < 3:
        print("too few rows to fit")
        return 1
    column = lambda name: np.array([float(r[name]) for r in usable])
    cart, pole = column("cart"), column("pole")
    pivot_x, pivot_y, end_x, end_y = column("pivot_x"), column("pivot_y"), column("end_x"), column("end_y")

    print(f"\ntruth covered: cart {cart.min():+.4f} .. {cart.max():+.4f} m, "
          f"pole {math.degrees(pole.min()):+.2f} .. {math.degrees(pole.max()):+.2f} deg")

    # Cart from Pivot x.
    (slope, intercept), residual = fit(pivot_x, cart)
    print(f"\ncart = {slope:.6f} x pivot_x {intercept:+.6f} m"
          f"  (1 px of Pivot x = {abs(slope) / WIDTH * 1000:.3f} mm)")
    print(f"  residual RMS {np.sqrt(np.mean(residual**2)) * 1000:.3f} mm, max {np.abs(residual).max() * 1000:.3f} mm")
    _, residual2 = fit(pivot_x, cart, 2)
    print(f"  (a quadratic fit: RMS {np.sqrt(np.mean(residual2**2)) * 1000:.3f} mm)")
    print(f"  pivot_y {pivot_y.min():.5f} .. {pivot_y.max():.5f} "
          f"({(pivot_y.max() - pivot_y.min()) * HEIGHT:.2f} px of vertical spread)")
    for label, metres in [("slide end", SLIDE_END_M), ("end-crash margin", SLIDE_END_M - MARGIN_M)]:
        xs = sorted((m - intercept) / slope for m in (-metres, metres))
        print(f"  {label} +-{metres:.4f} m -> pivot_x {xs[0]:.5f} .. {xs[1]:.5f} (extrapolated past the covered range)"
              if metres > max(abs(cart.min()), abs(cart.max())) else
              f"  {label} +-{metres:.4f} m -> pivot_x {xs[0]:.5f} .. {xs[1]:.5f}")

    # Pole from the Pivot-to-End direction, in pixels.
    angle = np.arctan2((end_x - pivot_x) * WIDTH, (end_y - pivot_y) * HEIGHT)
    (k, c), residual = fit(angle, pole)
    print(f"\npole = {k:.5f} x image angle {c:+.5f} rad"
          f"  (image angle {math.degrees(angle.min()):+.2f} .. {math.degrees(angle.max()):+.2f} deg)")
    print(f"  residual RMS {math.degrees(np.sqrt(np.mean(residual**2))):.3f} deg, "
          f"max {math.degrees(np.abs(residual).max()):.3f} deg")
    length_px = np.hypot((end_x - pivot_x) * WIDTH, (end_y - pivot_y) * HEIGHT)
    print(f"  Pivot-to-End length {length_px.min():.1f} .. {length_px.max():.1f} px")
    return 0


if __name__ == "__main__":
    sys.exit(main())
