//! Fixed facts, copied from the controller's `constants.rs` where the bench
//! firmware does what the controller does.

/// How long ENA must be set before DIR changes, ns: the DM542's t1, at least
/// 5 µs (the controller's `constants.rs`). Nothing on the bench reads ENA, but
/// the step generator is built exactly as the controller's.
pub(crate) const ENA_LEAD_NS: u32 = 5_000;

/// How long DIR must be set before STEP's next pulse, ns: the DM542's t2.
pub(crate) const DIR_SETUP_NS: u32 = 5_000;

/// The DM542's shortest STEP on and off times, ns (manual, section 10, t3
/// and t4).
pub(crate) const STEP_MIN_NS: u32 = 2_500;

/// What the board sends down USB at every start.
pub(crate) const LINE_READY: &[u8] = b"READY\n";

/// The end of every line on the USB line, both ways.
pub(crate) const LINE_END: &[u8] = b"\n";

/// The longest line the board takes over USB, bytes.
pub(crate) const LINE_MAX: usize = 32;

/// The PC's question: is the board running, and where can it be reached?
pub(crate) const LINE_STATUS: &[u8] = b"STATUS";

/// The answers to `STATUS`: `OK STATUS joining`, `OK STATUS running <address>
/// <port>`, or `OK STATUS stopped <why>`.
pub(crate) const REPLY_STATUS: &[u8] = b"OK STATUS ";
pub(crate) const STATUS_JOINING: &[u8] = b"joining";
pub(crate) const STATUS_RUNNING: &[u8] = b"running ";

/// The most of one AT reply the board keeps, bytes.
pub(crate) const REPLY_MAX: usize = 256;

/// The two lines that end an AT reply: the command was done, or refused.
pub(crate) const REPLY_OK: &[u8] = b"OK\r\n";
pub(crate) const REPLY_ERROR: &[u8] = b"ERROR\r\n";

/// The join's AT commands that carry no setting.
pub(crate) const AT_HANDSHAKE: &[u8] = b"AT\r\n";
pub(crate) const AT_SOFTRESETWIFI: &[u8] = b"AT+SOFTRESETWIFI\r\n";
pub(crate) const AT_WIFIMODE_OFF: &[u8] = b"AT+WIFIMODE=0\r\n";
pub(crate) const AT_WIFIMODE_STATION: &[u8] = b"AT+WIFIMODE=1\r\n";
pub(crate) const AT_GETSTATUS: &[u8] = b"AT+GETSTATUS?\r\n";
pub(crate) const AT_IPSTA: &[u8] = b"AT+IPSTA=0\r\n";

/// Where the value starts in each reply the join reads.
pub(crate) const PROMPT_GETSTATUS: &[u8] = b"+GETSTATUS: ";
pub(crate) const PROMPT_IPSTA: &[u8] = b"+IPSTA: ";
pub(crate) const PROMPT_UDPBEGIN: &[u8] = b"+UDPBEGIN: ";

/// The join's status numbers: the bridge's `WIFI_ST_CONNECTED` and the two
/// failures it names (at_handler.h).
pub(crate) const WL_NETWORK_NOT_FOUND: u32 = 1;
pub(crate) const WL_CONNECTED: u32 = 3;
pub(crate) const WL_CONNECT_FAILED: u32 = 4;

/// How the bridge pushes a datagram, with push mode on:
/// `+UDPRX: <sock>,<len>|<len bytes>\r\n` (the interrupt bridge's release
/// notes).
pub(crate) const FRAME_PROMPT: &[u8] = b"+UDPRX: ";
pub(crate) const FRAME_HEADER_END: u8 = b'|';
pub(crate) const FRAME_END: &[u8] = b"\r\n";

/// The most of a datagram's payload the board keeps, bytes.
pub(crate) const DATAGRAM_MAX: usize = 32;

/// The longest address the board keeps, bytes: an IPv4 address,
/// `255.255.255.255`.
pub(crate) const ADDRESS_MAX: usize = 15;

/// A command's size, bytes: the sign, then the size as a u16, little-endian
/// (real-loop-dia.md, *Contracts*).
pub(crate) const COMMAND_SIZE: u32 = 3;

/// A command's sign byte: 0 negative, 1 positive. Any other is ignored on the
/// bench: there is no FAULT to enter.
pub(crate) const COMMAND_NEGATIVE: u8 = 0;
pub(crate) const COMMAND_POSITIVE: u8 = 1;

/// The bench's stand-in stop points, pulses either side of where the board
/// started: the controller caps every tick's rate by the soft limit, so the
/// bench does the same arithmetic, against ends too far away to reach. A
/// billion pulses is 15 km of rail, and 2 × m × 10⁹ still fits a u64.
pub(crate) const BENCH_STOP_POINT: i64 = 1_000_000_000;

/// Microseconds in a second.
pub(crate) const MICROS_PER_SECOND: u64 = 1_000_000;
