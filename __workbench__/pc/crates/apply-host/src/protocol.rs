//! The words the PC and the two boards exchange over USB. Each must match the
//! board's own copy, named beside it: change one, change both.

/// The end of every line, both ways, on both boards.
pub(crate) const LINE_END: u8 = b'\n';

/// The R3 sketch's line at every start (`apply_timer.ino`, `setup`).
pub(crate) const R3_READY: &str = "READY apply_timer";

/// The R3 command that starts timing, followed by the change threshold in
/// its counts and the timeout in µs, and the start of its answer, which
/// repeats them (`apply_timer.ino`, `handle`).
pub(crate) const R3_START: &str = "start";
pub(crate) const R3_STARTED: &str = "OK start";

/// The R3 command that stops timing, and the start of its answer, which is
/// followed by how many results it had no room for (`apply_timer.ino`,
/// `handle`).
pub(crate) const R3_STOP: &str = "stop";
pub(crate) const R3_STOPPED: &str = "OK stop ";

/// The R3's result kinds, each line's first field (`apply_timer.ino`,
/// `KIND_APPLIED`, `KIND_EARLY`, `KIND_NONE`).
pub(crate) const KIND_APPLIED: &str = "A";
pub(crate) const KIND_EARLY: &str = "E";
pub(crate) const KIND_NONE: &str = "N";

/// The R3's clock: Timer1 at 16 MHz with no prescaler (`apply_timer.ino`,
/// `TICKS_PER_US`).
pub(crate) const R3_TICKS_PER_SECOND: f64 = 16e6;
pub(crate) const R3_TICKS_PER_US: f64 = 16.0;

/// The R4's question and its answers (`apply-firmware`'s `constants.rs`,
/// `LINE_STATUS`, `REPLY_STATUS` and `STATUS_RUNNING`; its `main.rs`,
/// `stopped`).
pub(crate) const R4_STATUS: &str = "STATUS";
pub(crate) const R4_RUNNING: &str = "OK STATUS running ";
pub(crate) const R4_STOPPED: &str = "OK STATUS stopped ";
