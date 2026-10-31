use crate::{experiment::JumpLimit, halt::Halt, switch::Switch};

/// How the last action ended, for the page and the LED.
#[derive(Clone, Copy, Debug)]
pub enum Outcome {
    /// Homed or moved to the centre, holding at this pulse.
    Centred { position: i32 },
    /// A jog ended at this pulse: after its full distance, or early at a
    /// switch that tripped.
    Jogged { position: i32, at_switch: Option<Switch> },
    /// The jump-limit experiment ended with this m.
    Measured(JumpLimit),
    Enabled,
    /// The driver is off and the cart free, so the count no longer says
    /// where it is.
    Disabled,
    Halted(Halt),
}
