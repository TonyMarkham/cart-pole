use embedded_hal::digital::PinState;

/// A direction of travel along the rail, by the level on D5 (DIR). Which
/// physical end each leads to depends on the motor's wiring. The step
/// generator sets DIR itself, from the sign of the rate it is given, so a
/// direction becomes a signed rate (`rate`), never a write to the pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    High,
    Low,
}

impl Direction {
    /// The direction positive rates run: the step generator's forward level
    /// (`pins.rs`, `DIR`). A choice that keeps `calibration.toml`'s levels
    /// meaning what they say; the Run's positive direction is still open
    /// (`real-loop-dia.md`, *DIR's level for a positive command*).
    pub const FORWARD: Direction = Direction::High;

    /// The level on D5.
    pub const fn level(self) -> PinState {
        match self {
            Direction::High => PinState::High,
            Direction::Low => PinState::Low,
        }
    }

    /// `speed` pulses/s as the signed rate that runs this way.
    pub const fn rate(self, speed: i32) -> i32 {
        match (self, Self::FORWARD) {
            (Direction::High, Direction::High) | (Direction::Low, Direction::Low) => speed,
            _ => -speed,
        }
    }
}
