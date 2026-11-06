use crate::{config::DIR_POSITIVE, constants::DIR_SETUP_NS};

use unor4_hal::{
    gpio::pin_map::{self, D5},
    step::{DirPinBuilder, DirSetup},
};

/// DIR on D5. The step generator sets it from each rate's sign, so the
/// firmware never writes the pin: a positive rate runs with D5 at
/// `DIR_POSITIVE`, `config.toml`'s level for a positive command (issue 4).
pub(crate) const DIR: DirSetup<D5> = DirPinBuilder::new()
    .with_pin(pin_map::d5)
    .with_setup(DIR_SETUP_NS)
    .with_forward(DIR_POSITIVE)
    .build();
