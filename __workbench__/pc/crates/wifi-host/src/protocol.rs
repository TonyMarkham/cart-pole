//! The words the PC and the R4's `wifi-firmware` exchange over USB. Each must
//! match the firmware's copy in its `src/constants.rs`, named beside it.

/// The end of every line, both ways (`LINE_END`).
pub(crate) const LINE_END: u8 = b'\n';

/// The R4's question and its answers (`LINE_STATUS`; `REPLY_STATUS` with
/// `STATUS_RUNNING`, then `<address> <port>`; or `stopped <why>`, its
/// `main.rs`, `stopped`).
pub(crate) const R4_STATUS: &str = "STATUS";
pub(crate) const R4_RUNNING: &str = "OK STATUS running ";
pub(crate) const R4_STOPPED: &str = "OK STATUS stopped ";

/// A clock sync's prefix, both ways (`SYNC_PREFIX`). The request is
/// `SYNC <n> 00000000 00000000`, the answer `SYNC <n> <t1> <t2>`, each
/// field 8 hex digits: the same length both ways (`SYNC_LINE_LEN`, 31).
pub(crate) const SYNC_PREFIX: &str = "SYNC ";

/// The two stamps' placeholder in a request: 8 zeros each.
pub(crate) const SYNC_PLACEHOLDER: &str = "00000000 00000000";

/// A datagram's report: `PING <payload, 6 hex digits> <stamp, 8 hex digits>`
/// (`PING_PREFIX`).
pub(crate) const PING_PREFIX: &str = "PING ";

/// A report's payload field: 3 bytes as 2 hex digits each
/// (`BYTE_HEX_DIGITS`).
pub(crate) const PAYLOAD_HEX_DIGITS: usize = 6;

/// What separates a line's fields (`FIELD_SEPARATOR`).
pub(crate) const FIELD_SEPARATOR: char = ' ';

/// The R4's clock rate as designed, counts per second: GPT320 counting
/// PCLKD, 48 MHz from the HOCO (unor4-hal `clock.rs`, `HOCO_FREQ_HZ`). The
/// true rate is measured by the fit; this one only widens the 32-bit
/// stamps and converts the R4's few µs between `t1` and `t2`.
pub(crate) const R4_NOMINAL_HZ: f64 = 48e6;

/// The R4's stamps are 32 bits: they wrap every 2³² counts.
pub(crate) const R4_WRAP: f64 = 4_294_967_296.0;

/// The base the R4 writes its numbers in: hex, 16 (`hex_line.rs`).
pub(crate) const HEX_RADIX: u32 = 16;
