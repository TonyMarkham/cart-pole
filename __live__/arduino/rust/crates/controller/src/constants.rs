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

/// The most of one AT reply the board keeps, bytes. Replies the join reads,
/// such as `+GETSTATUS: 3`, are a few dozen bytes.
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

/// The most of a datagram's payload the board keeps, bytes. The longest text
/// message the plans name, `SET runtime_end_margin 99999`, is 27
/// (real-open-issues.md, issue 9). The rest is counted, not kept.
pub(crate) const DATAGRAM_MAX: usize = 32;

/// Where the sender's address and port start in their replies.
pub(crate) const PROMPT_UDPREMOTEIP: &[u8] = b"+UDPREMOTEIP: ";
pub(crate) const PROMPT_UDPREMOTEPORT: &[u8] = b"+UDPREMOTEPORT: ";

/// The longest sender's address the board keeps, bytes: an IPv4 address,
/// `255.255.255.255`.
pub(crate) const ADDRESS_MAX: usize = 15;

/// The `state` message (real-open-issues.md, issue 7, step 1). At 5 bytes it
/// can't be taken for a command.
pub(crate) const STATE_REQUEST: &[u8] = b"state";

/// The Orin's requests for a seek to each end and for the move to the centre,
/// and the message that ends a move: they must match the webserver's
/// `IDLER_REQUEST`, `MOTOR_REQUEST`, `CENTRE_REQUEST` and `STOP_REQUEST`
/// (`__live__/orin/rust/crates/webserver/src/constants.rs`). The move to the
/// centre may carry a speed after a space: `MoveToCentre 5000` (the
/// webserver's `calibrate/mod.rs`, `centre`).
pub(crate) const SEEK_IDLER_REQUEST: &[u8] = b"CalibrateIdlerTravelLimit";
pub(crate) const SEEK_MOTOR_REQUEST: &[u8] = b"CalibrateMotorTravelLimit";
pub(crate) const CENTRE_REQUEST: &[u8] = b"MoveToCentre";
pub(crate) const CENTRE_SPEED_SEPARATOR: u8 = b' ';
pub(crate) const STOP_REQUEST: &[u8] = b"Stop";

/// The switches by the pin names a seek's answer gives in `pin`, which the
/// webserver's calibrate page shows: the idler's end is D3, the motor's D2.
pub(crate) const PIN_IDLER: &str = "d3";
pub(crate) const PIN_MOTOR: &str = "d2";

/// A command's size, bytes: the sign, then the size as a u16, little-endian.
/// The board tells a command from a text message by size alone
/// (real-loop-dia.md, *Contracts*).
pub(crate) const COMMAND_SIZE: u32 = 3;

/// A command's sign byte: 0 negative, 1 positive (real-loop-dia.md,
/// *Contracts*, the command). Any other is a fault (the user, 2026-11-06).
pub(crate) const COMMAND_NEGATIVE: u8 = 0;
pub(crate) const COMMAND_POSITIVE: u8 = 1;

/// The soft limit's gap, pulses, between each end's cached trip position and
/// its stop point: the cart is brought to rest at a stop point, and passing
/// one is entering that end's protected zone, a fault. 1,000 pulses is 15 mm.
/// The default the user decided (2026-11-04, real-open-issues.md, issues 8 and
/// 11), kept in code by that decision, not in a toml. Setting it from the
/// Orin and keeping it in the data flash (issues 9 to 11) comes later.
pub(crate) const RUNTIME_END_MARGIN: i64 = 1_000;

/// The status numbers in an answer, with HTTP's meanings (RFC 9110, section
/// 15).
pub(crate) const STATUS_OK: u16 = 200;
pub(crate) const STATUS_BAD_REQUEST: u16 = 400;
pub(crate) const STATUS_TIMEOUT: u16 = 408;
pub(crate) const STATUS_CONFLICT: u16 = 409;

/// Room for one answer, bytes. The longest today is a seek's `already_there`
/// with the widest `pulses`, 4,294,967,295: 145 bytes.
pub(crate) const ANSWER_MAX: usize = 160;

/// Microseconds in a second, for the time a move takes to go some pulses.
pub(crate) const MICROS_PER_SECOND: u64 = 1_000_000;
