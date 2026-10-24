"""The graded reward and the end crash, from the pool alone.

The policy has to swing the pole up from hanging and keep the cart off the
supports, so each step is scored on both (synthetic-loop-dia.md, The reward
is graded; trainer-plan.md T3a, T4), from what the trainer sees:

- the pole part, (1 - cos theta) / 2: theta is the Pivot-to-End direction in
  the image, in pixels (x right, y down; 0 = End straight below Pivot),
  measured equal to the pole angle (trainer-plan.md, What the trainer
  sees), so the part is 0 hanging and 1 upright and rises all the way up;
- the cart part, 1 - u^2: u is the Pivot marker's x offset from the middle
  of the end-crash bounds over their half-span, 1 in the middle and 0 at a
  bound;
- reward = pole_weight x pole part + cart_weight x cart part.

The Pivot marker's x outside the bounds is the end crash: the step scores
the penalty instead, and the episode terminates. Each step is scored from
the newest pool entry with both Pivot and End detected; with none, the step
scores 0 and cannot crash.
"""

import math
from dataclasses import dataclass

from gym_settings import EndCrash, Reward
from pool import END, FRAME_HEIGHT, FRAME_WIDTH, PIVOT, Pool


@dataclass(frozen=True)
class Score:
    reward: float
    crashed: bool
    pole: float | None  # the pole part, 0 to 1; None with no usable entry
    cart: float | None  # the cart part, 1 to 0 inside the bounds


def score(pool: Pool, reward: Reward, end_crash: EndCrash) -> Score:
    entry = next((e for e in pool.entries if e.detected(PIVOT) and e.detected(END)), None)
    if entry is None:
        return Score(0.0, False, None, None)
    (pivot_x, pivot_y), (end_x, end_y) = entry.markers[PIVOT], entry.markers[END]
    theta = math.atan2((end_x - pivot_x) * FRAME_WIDTH, (end_y - pivot_y) * FRAME_HEIGHT)
    pole = (1 - math.cos(theta)) / 2
    middle = (end_crash.pivot_x_min + end_crash.pivot_x_max) / 2
    half_span = (end_crash.pivot_x_max - end_crash.pivot_x_min) / 2
    u = (pivot_x - middle) / half_span
    cart = 1 - u * u
    if not end_crash.pivot_x_min <= pivot_x <= end_crash.pivot_x_max:
        return Score(end_crash.penalty, True, pole, cart)
    return Score(reward.pole_weight * pole + reward.cart_weight * cart, False, pole, cart)
