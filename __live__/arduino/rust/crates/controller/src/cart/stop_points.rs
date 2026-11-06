use crate::{config::motion::JUMP_LIMIT, constants::MICROS_PER_SECOND};

use unor4_hal::step::TICK_US;

/// The stop points: `runtime_end_margin` pulses short of each end's cached
/// trip position, on the cart's count. While the Orin's commands move the
/// cart, the soft limit brings it to rest at a stop point and never past it,
/// and the count passing one is the cart entering that end's protected zone
/// (real-loop-dia.md, *The soft limit*; real-open-issues.md, issue 5). The
/// move to the centre brakes by the same rule onto one stop point, the centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StopPoints {
    /// The idler's stop point: the count may not go below it.
    idler: i64,
    /// The motor's stop point: the count may not go above it.
    motor: i64,
}

impl StopPoints {
    pub(crate) const fn new(idler: i64, motor: i64) -> Self {
        Self { idler, motor }
    }

    /// One stop point, `point`, from either side: heading for the motor's
    /// end it is the motor's stop point, heading for the idler's the idler's.
    pub(crate) const fn at(point: i64) -> Self {
        Self::new(point, point)
    }

    /// Whether `position` is past either stop point, inside a protected zone.
    pub(crate) const fn passed(&self, position: i64) -> bool {
        position < self.idler || position > self.motor
    }

    /// The soft limit on `next`, the rate the ramp asks for this tick, with
    /// `running`, the rate STEP runs at now, and the cart at `position`.
    ///
    /// Heading for an end, the rate is at most √(2 × m × d), d the pulses left
    /// to that end's stop point: braking as hard as m allows, the cart then
    /// reaches 0 at the stop point (real-loop-dia.md, *The soft limit*). Moving
    /// away from an end is never capped. Two bounds keep it short of the stop
    /// point (real-open-issues.md's plan, D3):
    /// - a rate queued now runs from the next tick (unor4-hal
    ///   `step/step_generator.rs`, `queue`), so d is taken one tick ahead:
    ///   d₁ = d − the pulses `running` covers in one tick;
    /// - one tick may not cover more than d₁, so the rate is at most d₁ ÷ the
    ///   tick.
    ///
    /// Both bounds and the curve are 0 once d₁ ≤ 0.
    pub(crate) fn cap(&self, next: i32, running: i32, position: i64) -> i32 {
        let left = match next.signum() {
            1 => self.motor - position,
            -1 => position - self.idler,
            _ => return 0,
        };
        // Pulses the running rate covers before a rate queued now takes over,
        // when it heads the same way.
        let ahead = if running.signum() == next.signum() {
            i64::from(running.unsigned_abs()) * i64::from(TICK_US) / MICROS_PER_SECOND as i64
        } else {
            0
        };
        let left = left - ahead;
        if left <= 0 {
            return 0;
        }
        let left = left.unsigned_abs();
        let by_curve = (2 * u64::from(JUMP_LIMIT) * left).isqrt();
        let by_tick = left * MICROS_PER_SECOND / u64::from(TICK_US);
        let cap = by_curve.min(by_tick);
        let size = u64::from(next.unsigned_abs()).min(cap);
        // At most |next|, an i32.
        let size = i32::try_from(size).unwrap_or(i32::MAX);
        next.signum() * size
    }
}
