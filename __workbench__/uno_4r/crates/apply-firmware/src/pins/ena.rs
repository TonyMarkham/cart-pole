use crate::constants::ENA_LEAD_NS;

use embedded_hal::digital::PinState;
use unor4_hal::{
    gpio::pin_map::{self, D6},
    step::{EnaPinBuilder, EnaSetup},
};

/// ENA on D6: a high pin leaves the driver's ENA input unlit, which enables
/// it; a low one lights it and frees the motor (__documentation__/control-design.md, ENA+).
pub(crate) const ENA: EnaSetup<D6> = EnaPinBuilder::new()
    .with_pin(pin_map::d6)
    .with_enabled(PinState::High)
    .with_lead(ENA_LEAD_NS)
    .build();
