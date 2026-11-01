use crate::{answer::write_answer, constants::STATUS_OK, switch::Switch};

use core::fmt::{self, Write};

/// A move that did what was asked. Every other ending is an `AppError`.
#[derive(Clone, Copy, Debug)]
pub enum Moved {
    /// A switch tripped after this many pulses. `pulses` from one switch to the
    /// other is the rail's span.
    Tripped { switch: Switch, pulses: u32 },
    /// The cart reached the centre after this many pulses.
    Centred { pulses: u32 },
}

impl Moved {
    pub fn write_answer(&self, out: &mut impl Write) -> fmt::Result {
        match *self {
            Moved::Tripped { switch, pulses } => write_answer(
                out,
                STATUS_OK,
                "tripped",
                "the cart stopped at that end's switch",
                Some(switch),
                pulses,
            ),
            Moved::Centred { pulses } => write_answer(
                out,
                STATUS_OK,
                "centred",
                "the cart reached the centre",
                None,
                pulses,
            ),
        }
    }
}
