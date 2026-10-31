/// A direction of travel along the rail. Minus is the way the homing's
/// first seek runs, DIR low, toward the first end it finds; once homed,
/// that end is pulse 0 and the count grows in Plus. Which physical end
/// Minus leads to depends on the motor's wiring, and homing does not
/// care: it stops at whichever switch trips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Minus,
    Plus,
}

impl Direction {
    /// One pulse's change to the count.
    pub const fn sign(self) -> i32 {
        match self {
            Direction::Minus => -1,
            Direction::Plus => 1,
        }
    }

    /// DIR's level for this direction: low for Minus.
    pub const fn dir_high(self) -> bool {
        matches!(self, Direction::Plus)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Direction::Minus => "minus",
            Direction::Plus => "plus",
        }
    }
}
