use crate::{config::stepper::PULSE_LOW_NS, constants::STEP_MIN_NS};

use unor4_hal::{
    gpio::pin_map::{self, D4},
    step::{NoDivider, PulPinBuilder, PulSetup},
};

// The HAL checks only its own limits on the LOW time, not the driver's.
const _: () = assert!(
    PULSE_LOW_NS >= STEP_MIN_NS,
    "config.toml: pulse_low_ns is under the DM542's 2,500 ns on time (t3)"
);

/// PUL on D4 (GPT320, 32-bit, output B): rests high, goes low for
/// `pulse_low_ns` per pulse, then stays high for at least the DM542's off
/// time (t4).
pub(crate) const PUL: PulSetup<D4, NoDivider> = PulPinBuilder::new()
    .with_pin(pin_map::d4)
    .with_timing(PULSE_LOW_NS, STEP_MIN_NS)
    .build();
