use unor4_hal::gpio::{Output, Pin, pin_map::D3};

/// D3, wired to the R3's D3 on the HAL's bench (unor4-hal `Validation.md`,
/// *All-in-one validation*: D3 ↔ D3): high while the processor is awake, low
/// while it sleeps in `WFI`. The R3 times each edge on its INT1 interrupt
/// (`__workbench__/uno_r3/awake_timer`). The rig's controller has its idler
/// switch on D3; the bench has no switches.
pub(crate) struct Marker {
    pin: Pin<D3, Output>,
}

impl Marker {
    /// Takes D3 as an output, high: the processor is awake when it starts.
    pub(crate) fn new(pin: Pin<D3, Output>) -> Self {
        let mut marker = Self { pin };
        marker.awake();
        marker
    }

    /// The processor is about to sleep.
    pub(crate) fn asleep(&mut self) {
        self.pin.set_low();
    }

    /// The processor has just woken.
    pub(crate) fn awake(&mut self) {
        self.pin.set_high();
    }
}
