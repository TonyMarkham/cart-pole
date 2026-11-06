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

/// The `state` message (real-open-issues.md, issue 7, step 1). At 5 bytes it
/// can't be taken for a command.
pub(crate) const STATE_REQUEST: &[u8] = b"state";

/// A command's size, bytes: the sign, then the size as a u16, little-endian.
/// The board tells a command from a text message by size alone
/// (real-loop-dia.md, *Contracts*).
pub(crate) const COMMAND_SIZE: u32 = 3;

/// The status numbers in an answer, with HTTP's meanings (RFC 9110, section
/// 15).
pub(crate) const STATUS_OK: u16 = 200;
pub(crate) const STATUS_CONFLICT: u16 = 409;

/// Room for one answer, bytes. The longest today, the 409 for a command, is
/// about 120, so 160 leaves room for `pin` and `pulses` once they carry values.
pub(crate) const ANSWER_MAX: usize = 160;
