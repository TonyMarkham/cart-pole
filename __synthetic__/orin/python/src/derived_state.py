"""The five numbers DeepMind's cart-pole policy reads, worked out from what the camera
saw (__synthetic__/trainer/python/dm_control_reward.md, Where the reward's numbers come
from; its note 1, get_observation).

DeepMind's simulator hands its policy these directly. The robot has no such sensor: it
knows only where the detector found Pivot and End, so here they come from the pool, the
last six frames' points in metres (calibrate-w-homography.md, Per frame).

  cart        the cart's position along the rail, m: Pivot's x less the calibration's
              centre stop, so 0 centred and negative to the left as the camera sees it
  cos_theta   how much of the pole points up: 1 straight up, 0 sideways, -1 hanging
  sin_theta   how much of it leans along the rail: positive leaning towards +x
  cart_speed  how fast the cart moves, m/s, positive towards +x
  pole_speed  how fast the pole turns, rad/s, positive from straight up towards +x:
              DeepMind's sense. This rig's MJCF joint counts the other way, so
              MuJoCo's qvel for it has the opposite sign (measured 2026-11-01)

theta is the pole's angle from straight up, in radians, read from the arrow from Pivot
to End: theta = atan2(dx, dz). Hanging, dx is 0 and dz is -0.45 m, so theta is pi.

A single frame holds no speed, so each speed is a difference between two of the pool's
entries: the newest, and the next older that saw what the speed needs, divided by the
time between them. It is the average speed over that gap, so it describes a moment
about half a gap old: 21 ms at the mean gap of 41.5 ms (synthetic-loop-dia.md).

A value that needs a marker the frames did not see is None. What the policy should get
instead is not decided (calibrate-w-homography.md, Open).
"""

import math
from dataclasses import asdict, dataclass

# A point with no detection (synthetic-loop-dia.md, Contracts, the points).
NOT_DETECTED = (-1.0, -1.0)


@dataclass(frozen=True)
class DerivedState:
    cart: float | None
    cos_theta: float | None
    sin_theta: float | None
    cart_speed: float | None
    pole_speed: float | None

    def to_json(self) -> dict:
        return asdict(self)


def _pivot(row: list[float]) -> tuple[float, float] | None:
    point = (row[1], row[2])
    return None if point == NOT_DETECTED else point


def _theta(row: list[float]) -> float | None:
    """The pole's angle in one pool entry, or None without both Pivot and End."""
    pivot, end = _pivot(row), (row[3], row[4])
    if pivot is None or end == NOT_DETECTED:
        return None
    return math.atan2(end[0] - pivot[0], end[1] - pivot[1])


def _seconds_between(newer: list[float], older: list[float]) -> float:
    return (newer[0] - older[0]) / 1000


def derive(pool: list[list[float]], centre_x: float) -> DerivedState:
    """The five numbers for the newest frame. `pool` is the rows the Orin replies with,
    newest first: [age_ms, pivot_x, pivot_z, end_x, end_z]. `centre_x` is Pivot's x at
    the centre stop (fit.Homography)."""
    newest, older = pool[0], pool[1:]

    cart = cart_speed = None
    pivot = _pivot(newest)
    if pivot is not None:
        cart = pivot[0] - centre_x
        for row in older:
            seen = _pivot(row)
            if seen is not None:
                cart_speed = (pivot[0] - seen[0]) / _seconds_between(newest, row)
                break

    cos_theta = sin_theta = pole_speed = None
    theta = _theta(newest)
    if theta is not None:
        cos_theta, sin_theta = math.cos(theta), math.sin(theta)
        for row in older:
            seen_theta = _theta(row)
            if seen_theta is not None:
                # theta jumps from +pi to -pi as the pole swings through hanging, so
                # the turn is taken the short way round. That is right while the pole
                # turns under half a turn between the two entries: under 60 rad/s
                # across the longest gap between frames, 52 ms, and under 12 rad/s
                # across five of them, when four frames in a row missed a marker.
                turn = theta - seen_theta
                turn = math.atan2(math.sin(turn), math.cos(turn))
                pole_speed = turn / _seconds_between(newest, row)
                break

    return DerivedState(cart, cos_theta, sin_theta, cart_speed, pole_speed)
