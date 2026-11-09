use ra4m1::GPT166;
use unor4_hal::{
    elc::ElcF,
    wake::{WakeSetup, WakeTimerBuilder},
};

/// The move timer: a second one-shot wait, for a move's own time limits, such
/// as a seek's `seek_timeout_ms`. The wake timer can't time those: while the
/// cart moves, it times each datagram's frame from the WiFi chip, and one
/// timer runs one wait at a time. GPT166 is free, and it stops itself through
/// ELC input F, since the step generator holds B, C and D and the wake timer
/// E. GPT166 is a 16-bit channel too, so one wait lasts at most 1.398 s, and
/// `Deadline` chains them for a longer limit.
pub(crate) const MOVE_TIMER: WakeSetup<GPT166, ElcF> = WakeTimerBuilder::new()
    .with_timer::<GPT166>()
    .with_input::<ElcF>()
    .build();
