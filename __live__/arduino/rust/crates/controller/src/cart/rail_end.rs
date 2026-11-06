use crate::constants::{PIN_IDLER, PIN_MOTOR};

/// One end of the rail, each with its limit switch: the idler's on D3, the
/// motor's on D2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RailEnd {
    Idler,
    Motor,
}

impl RailEnd {
    /// The end across the rail.
    pub(crate) const fn other(self) -> Self {
        match self {
            Self::Idler => Self::Motor,
            Self::Motor => Self::Idler,
        }
    }

    /// The rate that drives the cart towards this end at `speed` pulses/s. A
    /// positive rate runs towards the motor's end (real-loop-dia.md, *DIR's
    /// level for a positive command*), so the idler's end is negative.
    pub(crate) const fn rate(self, speed: i32) -> i32 {
        match self {
            Self::Idler => -speed,
            Self::Motor => speed,
        }
    }

    /// The end's switch by the pin name a seek's answer gives in `pin`.
    pub(crate) const fn pin(self) -> &'static str {
        match self {
            Self::Idler => PIN_IDLER,
            Self::Motor => PIN_MOTOR,
        }
    }
}
