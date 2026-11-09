mod counter;
mod tick;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    pins::{dir::DIR, ena::ENA, pul::PUL},
    stepper::{counter::COUNTER, tick::TICK},
};

use ra4m1::{GPT164, GPT165, GPT321};
use unor4_hal::{
    gpio::pin_map::{D4, D5, D6},
    step::{NoDivider, StepGeneratorBuilder, StepSetup},
};

/// The controller's step generator, unchanged: STEP on D4 (to the R3's D8 on
/// the bench), DIR on D5 (to the R3's D7), ENA on D6 (not wired).
pub(crate) const STEPPER: StepSetup<D4, NoDivider, D5, GPT321, GPT164, GPT165, D6> =
    StepGeneratorBuilder::new()
        .with_pul(PUL)
        .with_dir(DIR)
        .with_ena(ENA)
        .with_counter(COUNTER)
        .with_tick(TICK)
        .build();
