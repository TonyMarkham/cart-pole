use crate::{
    direction::Direction,
    settings::{DIR_SETUP_NS, PULSE_LOW_NS, TRIP_US},
    stopwatch::Stopwatch,
    switch::Switch,
};

use core::fmt::{self, Write};
use embedded_hal::{delay::DelayNs, digital::PinState};
use unor4_hal::{
    delay::Delay,
    gpio::{Input, InputPullUp, Output, Pin},
};

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
    /// D5 = P107: DIR.
    d5: Pin<'1', 7, Output>,
    /// D6 = P111: ENA; low disables the driver, high leaves it enabled.
    d6: Pin<'1', 11, Output>,
    /// ICLK in Hz, for timing a seek.
    sysclk: u32,
}

impl Pins {
    /// Claims the pins. Every output starts high, so no driver input is lit:
    /// the driver is enabled and the motor holds still until the first pulse.
    /// The level is set before the pin starts driving, so the transistors'
    /// pull-ups' unlit state carries straight on.
    pub fn new(
        d2: Pin<'1', 4, Input>,
        d3: Pin<'1', 5, Input>,
        d4: Pin<'1', 6, Input>,
        d5: Pin<'1', 7, Input>,
        d6: Pin<'1', 11, Input>,
        sysclk: u32,
    ) -> Self {
        Self {
            // The board's own pull-ups are the switch lines' only ones
            // (control-design.md).
            d2: d2.into_input_pullup(),
            d3: d3.into_input_pullup(),
            d4: d4.into_output_in_state(PinState::High),
            d5: d5.into_output_in_state(PinState::High),
            d6: d6.into_output_in_state(PinState::High),
            sysclk,
        }
    }

    pub fn sysclk(&self) -> u32 {
        self.sysclk
    }

    /// Whether `switch` is pressed. The NC contact opens when the cart
    /// presses it, and so does a broken wire: both read high.
    pub fn pressed(&self, switch: Switch) -> bool {
        match switch {
            Switch::D2 => self.d2.is_high(),
            Switch::D3 => self.d3.is_high(),
        }
    }

    /// Whether `switch` reads pressed at every reading for `trip_us`, so a
    /// spike on its wire is not taken for a pressed switch.
    pub fn pressed_steady(&self, switch: Switch) -> bool {
        let mut clock = Stopwatch::start(self.sysclk);
        let hold = u64::from(TRIP_US) * u64::from(self.sysclk / 1_000_000);
        while clock.cycles() < hold {
            if !self.pressed(switch) {
                return false;
            }
        }
        true
    }

    /// Sets DIR, and waits the DM542's setup time before the next pulse.
    pub fn set_direction(&mut self, delay: &mut Delay, direction: Direction) {
        match direction {
            Direction::High => self.d5.set_high(),
            Direction::Low => self.d5.set_low(),
        }
        delay.delay_ns(DIR_SETUP_NS);
    }

    /// One STEP pulse lasting `period_ns` (D4 low for `pulse_low_ns`). The
    /// loop's own work adds to it, so the real rate is a little under the asked
    /// one, never over it.
    pub fn pulse(&mut self, delay: &mut Delay, period_ns: u32) {
        self.d4.set_low();
        delay.delay_ns(PULSE_LOW_NS);
        self.d4.set_high();
        delay.delay_ns(period_ns.saturating_sub(PULSE_LOW_NS));
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
