use core::fmt::{self, Write};

use embedded_hal::digital::PinState;
use unor4_hal::gpio::{Input, InputPullUp, Output, Pin};

/// The rig's pins, as control-design.md wires them; the port and pin
/// numbers are the WiFi board's (unor4-hal `gpio/pins.rs`). Each output
/// reaches the DM542 driver through a PNP transistor, so a low pin lights
/// its input and a high one leaves it unlit (control-design.md, *Why the
/// transistors*).
pub struct Pins {
    /// D2 = P104: one switch's NC contact, high when pressed.
    d2: Pin<'1', 4, InputPullUp>,
    /// D3 = P105: the other switch, the same way.
    d3: Pin<'1', 5, InputPullUp>,
    /// D4 = P106: PUL; rests high, low for each pulse.
    d4: Pin<'1', 6, Output>,
    /// D5 = P107: DIR; low for Minus.
    d5: Pin<'1', 7, Output>,
    /// D6 = P111: ENA; low disables the driver, high leaves it enabled.
    d6: Pin<'1', 11, Output>,
}

impl Pins {
    /// Claims the pins. Every output starts high and stays high, so no
    /// driver input is lit and no pulse is ever sent: the cart cannot move.
    /// The level is set before the pin starts driving, so the transistors'
    /// pull-ups' unlit state carries straight on.
    pub fn new(
        d2: Pin<'1', 4, Input>,
        d3: Pin<'1', 5, Input>,
        d4: Pin<'1', 6, Input>,
        d5: Pin<'1', 7, Input>,
        d6: Pin<'1', 11, Input>,
    ) -> Self {
        Self {
            // The board's own pull-ups are the switch lines' only ones
            // (control-design.md).
            d2: d2.into_input_pullup(),
            d3: d3.into_input_pullup(),
            d4: d4.into_output_in_state(PinState::High),
            d5: d5.into_output_in_state(PinState::High),
            d6: d6.into_output_in_state(PinState::High),
        }
    }

    /// Every pin's level now as JSON, 1 = high, 0 = low: the levels and no
    /// meanings, which are the Orin page's. Inputs are read from the pin;
    /// outputs are the level the firmware drives (the port's output latch).
    pub fn write_levels(&self, out: &mut impl Write) -> fmt::Result {
        write!(
            out,
            "{{\"d2\":{},\"d3\":{},\"d4\":{},\"d5\":{},\"d6\":{}}}",
            u8::from(self.d2.is_high()),
            u8::from(self.d3.is_high()),
            u8::from(self.d4.is_set_high()),
            u8::from(self.d5.is_set_high()),
            u8::from(self.d6.is_set_high()),
        )
    }
}
