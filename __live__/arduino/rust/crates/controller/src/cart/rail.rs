use crate::{
    cart::{rail_end::RailEnd, stop_points::StopPoints},
    constants::RUNTIME_END_MARGIN,
};

/// The rail as the seeks found it: where each end's switch stopped the cart,
/// on the cart's own count of its position. Known once a seek has reached
/// each end, so the move to the centre can run from wherever the cart is
/// (real-arduino-plan.md, A7, the cached rail). A reset forgets it, since the
/// count starts again at every start.
pub(crate) struct Rail {
    idler: Option<i64>,
    motor: Option<i64>,
}

impl Rail {
    pub(crate) const fn new() -> Self {
        Self {
            idler: None,
            motor: None,
        }
    }

    /// A seek reached `end`, and the cart stopped there at `position`.
    pub(crate) fn record(&mut self, end: RailEnd, position: i64) {
        match end {
            RailEnd::Idler => self.idler = Some(position),
            RailEnd::Motor => self.motor = Some(position),
        }
    }

    /// Forgets both ends: the count may no longer match where the cart is.
    pub(crate) fn forget(&mut self) {
        *self = Self::new();
    }

    /// Half way between the two ends, or None until a seek has reached each.
    pub(crate) fn centre(&self) -> Option<i64> {
        Some(midpoint(self.idler?, self.motor?))
    }

    /// The stop points, `runtime_end_margin` inside each end, or None until a
    /// seek has reached each. The idler's end is the lower count, since a
    /// positive rate runs towards the motor's end.
    pub(crate) fn stop_points(&self) -> Option<StopPoints> {
        Some(StopPoints::new(
            self.idler? + RUNTIME_END_MARGIN,
            self.motor? - RUNTIME_END_MARGIN,
        ))
    }

    /// `position` counted from the idler's end, for an answer's `pulses`, or 0
    /// before that end is known or at or below it.
    pub(crate) fn pulses_from_idler(&self, position: i64) -> u32 {
        self.idler
            .and_then(|idler| u32::try_from(position - idler).ok())
            .unwrap_or(0)
    }
}

/// Half way from `a` to `b`, rounded towards `a`. The two are at most the
/// span apart, about 60,000 pulses, so nothing overflows.
const fn midpoint(a: i64, b: i64) -> i64 {
    a + (b - a) / 2
}
