use crate::direction::Direction;

/// The two signals the firmware gives the DM542 for one command
/// (synthetic-loop-dia.md, 20): DIR from the sign, STEP rate (the pulses
/// on the DM542's PUL input, control-design.md) from the
/// magnitude, in pulses per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dm542Signals {
    pub dir: Direction,
    pub step_rate: u32,
}

impl Dm542Signals {
    /// The signals for a pulse rate already clamped to the top rate. DIR
    /// is `Negative` only for a negative rate; at rate 0 no pulse is sent,
    /// so DIR has no effect.
    pub const fn from_rate(rate: i32) -> Dm542Signals {
        let dir = if rate < 0 {
            Direction::Negative
        } else {
            Direction::Positive
        };
        Dm542Signals {
            dir,
            step_rate: rate.unsigned_abs(),
        }
    }
}
