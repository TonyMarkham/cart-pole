//! The words the PC and the two boards exchange over USB. Each must match the
//! board's own copy, named beside it: change one, change both.

/// The end of every line, both ways, on both boards (`awake_timer.ino`,
/// `Serial.println`; `bench-firmware`'s `constants.rs`, `LINE_END`).
pub(crate) const LINE_END: u8 = b'\n';

/// The R3 sketch's line at every start (`awake_timer.ino`, `setup`).
pub(crate) const R3_READY: &str = "READY awake_timer";

/// The R3 command that resets the histogram and starts timing, followed by
/// the bin width in µs and the number of bins (`awake_timer.ino`, `handle`,
/// its `sscanf` pattern `start %u %u`).
pub(crate) const R3_START: &str = "start";

/// The start of the R3's answer to a `start` it took; the same two numbers
/// follow (`awake_timer.ino`, `handle`, `OK start `).
pub(crate) const R3_STARTED: &str = "OK start";

/// The R3 command that stops timing and sends the report (`awake_timer.ino`,
/// `handle`, `"report"`).
pub(crate) const R3_REPORT: &str = "report";

/// The R3 report's last line (`awake_timer.ino`, `report`).
pub(crate) const R3_END: &str = "end";

/// The R3 report's keys, one per line, each followed by its number
/// (`awake_timer.ino`, `report`, and the header's list of them).
pub(crate) const KEY_TICK_NS: &str = "tick_ns";
pub(crate) const KEY_COUNT: &str = "count";
pub(crate) const KEY_MISSED: &str = "missed";
pub(crate) const KEY_OVER: &str = "over";
pub(crate) const KEY_MIN_TICKS: &str = "min_ticks";
pub(crate) const KEY_MAX_TICKS: &str = "max_ticks";
pub(crate) const KEY_SUM_TICKS: &str = "sum_ticks";
pub(crate) const KEY_SPAN_TICKS: &str = "span_ticks";
pub(crate) const KEY_BIN_US: &str = "bin_us";
/// A bin's line: `bin <index> <count>`.
pub(crate) const KEY_BIN: &str = "bin";

/// The R4's question (`bench-firmware`'s `constants.rs`, `LINE_STATUS`).
pub(crate) const R4_STATUS: &str = "STATUS";

/// The R4's answers to it: `REPLY_STATUS` followed by `STATUS_RUNNING` and
/// then `<address> <port>` (`bench-firmware`'s `constants.rs` and
/// `run/runner.rs`, `answer_status`), or by `stopped <why>` (its `main.rs`,
/// `stopped`).
pub(crate) const R4_RUNNING: &str = "OK STATUS running ";
pub(crate) const R4_STOPPED: &str = "OK STATUS stopped ";
