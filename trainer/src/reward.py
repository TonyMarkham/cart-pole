"""The graded reward and the end crash, from the pool alone.

The policy has to swing the pole up from hanging, hold it there and keep
the cart off the supports, so each step is scored on both
(synthetic-loop-dia.md, The reward is graded, The pole pays only when it is
slow; trainer-plan.md T3a, T4), from what the trainer sees:

- the pole part, height x exp(-(omega / pole_speed)^2): theta is the
  Pivot-to-End direction in the image, in pixels (x right, y down; 0 = End
  straight below Pivot), measured equal to the pole angle (trainer-plan.md,
  What the trainer sees). The height is the distance from upright,
  d = pi - |theta|, on a log scale (synthetic-loop-dia.md, The pole's
  height is scored on a log scale): 1 - ln(d / band) / ln(pi / band), and
  1 within upright_band of upright, so it is 0 hanging and 1 upright, and
  every halving of d pays the same (0.19 at a 5 degree band). A high swing
  turning back below the top pays far less than upright, which
  (1 - cos theta) / 2 did not (0.93 at 150 degrees,
  trainer-observations.md). omega is the pole's turning speed, rad/s, the
  change in theta between the newest two entries with both Pivot and End
  over the gap between their ages. A still pole keeps its height in full,
  and a fast one earns almost nothing, so spinning the pole round no
  longer pays (the first run's spin averaged 0.5 for free,
  trainer-observations.md);
- the cart part, 1 - u^2: u is the Pivot marker's x offset from the middle
  of the end-crash bounds over their half-span, 1 in the middle and 0 at a
  bound;
- reward = pole_weight x pole part + cart_weight x cart part.

The Pivot marker's x outside the bounds is the end crash: the step scores
the penalty instead, and the episode terminates. Each step is scored from
the newest pool entry with both Pivot and End detected; with none, the step
scores 0 and cannot crash. With only one such entry the speed is unknown,
and the pole part is 0.
"""

import math
from dataclasses import dataclass

from gym_settings import EndCrash, Reward
from pool import END, FRAME_HEIGHT, FRAME_WIDTH, PIVOT, Entry, Pool


@dataclass(frozen=True)
class Score:
    reward: float
    crashed: bool
    pole: float | None  # the plain height, (1 - cos theta) / 2, 0 to 1, for the logs; None with no usable entry
    cart: float | None  # the cart part, 1 to 0 inside the bounds
    speed: float | None  # omega, rad/s; None with fewer than two usable entries


def angle(entry: Entry) -> float:
    (pivot_x, pivot_y), (end_x, end_y) = entry.markers[PIVOT], entry.markers[END]
    return math.atan2((end_x - pivot_x) * FRAME_WIDTH, (end_y - pivot_y) * FRAME_HEIGHT)


def upright_height(theta: float, band_deg: float) -> float:
    """The pole's height on the log scale: 0 hanging, 1 within band_deg of
    upright, each halving of the distance from upright paying the same."""
    band = math.radians(band_deg)
    # atan2 gives theta in (-pi, pi], so d runs from 0 upright to pi hanging.
    d = math.pi - abs(theta)
    if d <= band:
        return 1.0
    return 1 - math.log(d / band) / math.log(math.pi / band)


def score(pool: Pool, reward: Reward, end_crash: EndCrash) -> Score:
    usable = [e for e in pool.entries if e.detected(PIVOT) and e.detected(END)]
    if not usable:
        return Score(0.0, False, None, None, None)
    entry = usable[0]
    theta = angle(entry)
    pole = (1 - math.cos(theta)) / 2
    speed = None
    slow = 0.0
    if len(usable) > 1:
        # The turn between the two entries, wrapped into (-pi, pi]: it is
        # misread only past half a turn in one gap, which at the pool's
        # gaps (at most rnd_delay_max + 2 ms, 17 ms now) is 185 rad/s; the
        # first run's spin peaked at 27.
        turn = theta - angle(usable[1])
        turn = math.atan2(math.sin(turn), math.cos(turn))
        speed = turn / ((entry.age_ms - usable[1].age_ms) / 1000)
        slow = math.exp(-((speed / reward.pole_speed) ** 2))
    pivot_x = entry.markers[PIVOT][0]
    middle = (end_crash.pivot_x_min + end_crash.pivot_x_max) / 2
    half_span = (end_crash.pivot_x_max - end_crash.pivot_x_min) / 2
    u = (pivot_x - middle) / half_span
    cart = 1 - u * u
    if not end_crash.pivot_x_min <= pivot_x <= end_crash.pivot_x_max:
        return Score(end_crash.penalty, True, pole, cart, speed)
    height = upright_height(theta, reward.upright_band)
    return Score(reward.pole_weight * height * slow + reward.cart_weight * cart, False, pole, cart, speed)
