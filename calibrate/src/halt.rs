use core::fmt::{self, Write};

use crate::{direction::Direction, settings::SEEK_TIMEOUT_MS, switch::Switch};

/// Why an action stopped short.
#[derive(Clone, Copy, Debug)]
pub enum Halt {
    /// D8 read low. It was the DRV8825's FAULT input; with the DM542 it
    /// is unconnected and its pull-up holds it high, so a low D8 means
    /// something is wired to it.
    DriverFault { position: i32 },
    /// A seek ran `seek_timeout_ms` without any switch tripping.
    NoSwitch { direction: Direction },
    /// A switch was already pressed when the homing began, so the first
    /// seek cannot tell which way leads off it.
    PressedAtStart { switch: Switch },
    /// Both ends of the rail tripped the same pin: the switches' wiring
    /// is wrong.
    SameSwitch { switch: Switch },
    /// A switch tripped during a fast move, which stays well clear of
    /// both ends: steps were lost, or the switch is faulty.
    SwitchOnTheWay { switch: Switch, position: i32 },
    /// A return to the experiment's reference switch tripped the other
    /// one: the cart is not where the count says.
    WrongReference { switch: Switch },
    /// The page's STOP was pressed.
    Stopped { position: i32 },
    /// Go to centre or the experiment was asked for before the cart was
    /// homed.
    NotHomed,
}

impl Halt {
    /// Whether the halt switches the driver off. A fault or a lost count
    /// does: the cart's place is no longer known. The STOP button, a
    /// switch pressed at the start and an unhomed Go to centre leave the
    /// driver holding, so the count stays good and a jog can follow.
    pub const fn frees_cart(&self) -> bool {
        matches!(
            self,
            Halt::DriverFault { .. }
                | Halt::NoSwitch { .. }
                | Halt::SameSwitch { .. }
                | Halt::SwitchOnTheWay { .. }
                | Halt::WrongReference { .. }
        )
    }

    /// The halt in words, for the serial port and the WiFi page alike.
    /// Positions count from the first end once it has tripped, and from
    /// the power-up place before that.
    pub fn describe(&self, out: &mut impl Write) -> fmt::Result {
        match self {
            Halt::DriverFault { position } => write!(
                out,
                "D8 read low at pulse {position}; it is unconnected with the DM542 (no fault line), so check what is wired to it"
            ),
            Halt::NoSwitch { direction } => write!(
                out,
                "no switch tripped within {SEEK_TIMEOUT_MS} ms moving {}: check the switches' wiring, or whether the motor is stepping at all",
                direction.name()
            ),
            Halt::PressedAtStart { switch } => write!(
                out,
                "the {} switch is pressed, so the homing cannot tell which way leads off it: jog the cart off it, then press Calibrate",
                switch.name()
            ),
            Halt::SameSwitch { switch } => write!(
                out,
                "both ends tripped {}: the switches' wiring is wrong; each end needs its own pin",
                switch.name()
            ),
            Halt::SwitchOnTheWay { switch, position } => write!(
                out,
                "the {} switch tripped at pulse {position} during a fast move: steps were lost, or the switch is faulty",
                switch.name()
            ),
            Halt::WrongReference { switch } => write!(
                out,
                "the return to the reference switch tripped {} instead: the cart is not where the count says",
                switch.name()
            ),
            Halt::Stopped { position } => {
                write!(out, "STOP pressed; stopped at pulse {position}, the driver holding the cart")
            }
            Halt::NotHomed => write!(out, "not homed yet: press Calibrate first"),
        }
    }
}
