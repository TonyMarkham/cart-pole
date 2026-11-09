use unor4_hal::gpio::{Output, Pin, pin_map::D3};

/// D3, wired to the R3's D3 on the HAL's bench (unor4-hal `Validation.md`,
/// *All-in-one validation*: D3 ↔ D3): flipped the moment the processor has
/// a command whole, so each edge marks one command's arrival for the R3,
/// which times it on its INT1 interrupt (`__workbench__/uno_r3/
/// apply_timer`). Flipping, not pulsing: one write per command, and the R3
/// takes either edge.
pub(crate) struct Mark {
    pin: Pin<D3, Output>,
    high: bool,
}

impl Mark {
    /// Takes D3 as an output, low.
    pub(crate) fn new(mut pin: Pin<D3, Output>) -> Self {
        pin.set_low();
        Self { pin, high: false }
    }

    /// Marks one command: D3 changes level (unor4-hal `gpio/pin.rs`,
    /// `set_high`, `set_low`: one atomic port write each).
    pub(crate) fn flip(&mut self) {
        self.high = !self.high;
        if self.high {
            self.pin.set_high();
        } else {
            self.pin.set_low();
        }
    }
}
