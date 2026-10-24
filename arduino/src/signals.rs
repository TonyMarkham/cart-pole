/// The DRV8825's direction input, by the direction it moves the cart: a
/// positive command's direction or a negative one's. Which pin level is
/// which is not decided yet (synthetic-arduino-plan.md, *Not decided by
/// the diagram*), so the level is not named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Positive,
    Negative,
}

/// The two signals the firmware gives the DRV8825 for one command
/// (synthetic-loop-dia.md, 20): DIR from the sign, STEP rate from the
/// magnitude, in pulses per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drv8825Signals {
    pub dir: Direction,
    pub step_rate: u32,
}

impl Drv8825Signals {
    /// The signals for a pulse rate already clamped to the top rate. DIR
    /// is `Negative` only for a negative rate; at rate 0 no pulse is sent,
    /// so DIR has no effect.
    pub const fn from_rate(rate: i32) -> Drv8825Signals {
        let dir = if rate < 0 { Direction::Negative } else { Direction::Positive };
        Drv8825Signals { dir, step_rate: rate.unsigned_abs() }
    }
}
