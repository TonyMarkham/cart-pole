/// How long ENA must be set before DIR changes, ns: the DM542's t1, at least
/// 5 µs (manual, section 10; `control-design.md`, the ENA+ row). The step
/// generator refuses to move until it has passed since `enable()`.
pub(crate) const ENA_LEAD_NS: u32 = 5_000;

/// How long DIR must be set before STEP's next pulse, ns: the DM542's t2, at
/// least 5 µs (manual, section 10; `control-design.md`, the DIR+ row). The
/// step generator waits it after every DIR change.
pub(crate) const DIR_SETUP_NS: u32 = 5_000;

/// The DM542's shortest STEP on and off times, ns (manual, section 10, t3
/// and t4).
pub(crate) const STEP_MIN_NS: u32 = 2_500;

/// What the board tells `arduino-manager` at every start: it was reset, so it
/// knows nothing of the rail (arduino-manager's README, *The protocol*).
pub(crate) const LINE_READY: &[u8] = b"READY\n";

/// The end of every line on the USB line, both ways (arduino-manager's
/// README, *The protocol*).
pub(crate) const LINE_END: &[u8] = b"\n";

/// The longest line the board takes from the Pi, bytes. The longest the plans
/// name is `SET runtime_end_margin 99999`, 27 bytes (real-open-issues.md,
/// issue 9); 32 is what the WiFi side keeps of a datagram.
pub(crate) const LINE_MAX: usize = 32;

/// The manager's question: is the board alive, and in what state?
pub(crate) const LINE_STATUS: &[u8] = b"STATUS";

/// The start of the answer to `STATUS`; the state's name and `LINE_END`
/// follow.
pub(crate) const REPLY_STATUS: &[u8] = b"OK STATUS ";
