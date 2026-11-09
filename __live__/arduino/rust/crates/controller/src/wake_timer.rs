use ra4m1::GPT167;
use unor4_hal::{
    elc::ElcE,
    wake::{WakeSetup, WakeTimerBuilder},
};

/// The wake timer: a one-shot wait that ends a `WFI` once, for every wait with
/// a time limit, such as a reply from the WiFi chip. GPT167 is free, and it
/// stops itself through ELC input E, since the step generator holds B, C and D
/// (unor4-hal `wake/mod.rs`). GPT167 is a 16-bit channel, so one wait lasts at
/// most 1.398 s: a longer one is started again until it has all passed.
pub(crate) const WAKE_TIMER: WakeSetup<GPT167, ElcE> = WakeTimerBuilder::new()
    .with_timer::<GPT167>()
    .with_input::<ElcE>()
    .build();
