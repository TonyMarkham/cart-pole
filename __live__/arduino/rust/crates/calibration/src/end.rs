use crate::{direction::Direction, switch::Switch};

/// One end of the rail: its switch and the direction that leads to it.
#[derive(Clone, Copy, Debug)]
pub struct End {
    pub switch: Switch,
    pub direction: Direction,
}
