//! The firmware's fixed facts, each copied from a contract or worked out from
//! one. Any value someone might change between runs is in `calibration.toml`.

/// The requests answered, each the whole datagram; anything else is ignored.
/// This one asks for the pins' levels: `PINS_REQUEST` in the Orin's webserver.
pub(crate) const PINS_REQUEST: &[u8] = b"pins";

/// These two move the cart to one end of the rail, and are answered when it
/// has stopped (`seek`).
pub(crate) const IDLER_REQUEST: &[u8] = b"CalibrateIdlerTravelLimit";
pub(crate) const MOTOR_REQUEST: &[u8] = b"CalibrateMotorTravelLimit";

/// Moves the cart from a switch to the rail's centre, and is answered when it
/// has stopped. A space and a whole number after it give the speed in
/// pulses/s; with none it moves at `seek_speed`.
pub(crate) const CENTRE_REQUEST: &[u8] = b"MoveToCentre";

/// How the `Stop` datagram's frame ends, from the header's `|` to the line end:
/// `+UDPRX: <sock>,4|Stop\r\n`. It ends whichever move is under way, which is
/// then answered `stopped`. With no move under way, like any other datagram, it
/// is ignored. `STOP_REQUEST` in the Orin's webserver is the `Stop`.
pub(crate) const STOP_FRAME_END: &[u8] = b"|Stop\r\n";

/// A frame's start, and the end of its header.
pub(crate) const FRAME_PROMPT: &[u8] = b"+UDPRX: ";
pub(crate) const FRAME_HEADER_END: u8 = b'|';

/// The start of a request kept; longer than the longest request so that a
/// longer datagram does not pass for it.
pub(crate) const REQUEST_MAX: usize = 32;

/// Buffer for one AT reply.
pub(crate) const RESP_MAX: usize = 256;

/// Buffer for one AT command this firmware builds. The longest is
/// `AT+UDPBEGINPACKETIP=<sock>,<port>,<ip>`, under 50 bytes with a 15-byte
/// address and a 5-digit port.
pub(crate) const AT_COMMAND_MAX: usize = 64;

/// Room for the `pins` answer, `{"d2":0,"d3":0,"d4":1,"d5":1,"d6":1}`:
/// `PINS_ANSWER_MAX` in the Orin's webserver.
pub(crate) const PINS_ANSWER_MAX: usize = 64;

/// Room for a move's answer. The longest possible is 151 bytes: 83 for
/// `{"status":409,"result":"already_there","message":"","pin":"d3","pulses":4294967295}`
/// and 68 for `AppError::already_there`'s message.
pub(crate) const SEEK_ANSWER_MAX: usize = 160;

/// The status number in a move's answer, with HTTP's meanings (RFC 9110,
/// section 15). The board speaks UDP, not HTTP; the Orin's webserver is meant
/// to return the same number as a real HTTP status.
pub(crate) const STATUS_OK: u16 = 200;
pub(crate) const STATUS_BAD_REQUEST: u16 = 400;
pub(crate) const STATUS_REQUEST_TIMEOUT: u16 = 408;
pub(crate) const STATUS_CONFLICT: u16 = 409;
pub(crate) const STATUS_INTERNAL_ERROR: u16 = 500;

/// The longest short text kept from an AT reply: an address, `255.255.255.255`.
pub(crate) const TEXT_MAX: usize = 15;

/// "Connected": the bridge's `WIFI_ST_CONNECTED` (at_handler.h).
pub(crate) const WL_CONNECTED: u32 = 3;

/// The two other statuses the join reports by name: the network was not found,
/// and the connection failed, usually on the password.
pub(crate) const WL_NETWORK_NOT_FOUND: u32 = 1;
pub(crate) const WL_CONNECT_FAILED: u32 = 4;

/// DEMCR.TRCENA, bit 24: powers the DWT, so its cycle counter runs
/// (Armv7-M Architecture Reference Manual, DEMCR; the cortex-m crate's
/// `DCB_DEMCR_TRCENA` is the same bit).
pub(crate) const DEMCR_TRCENA: u32 = 1 << 24;

/// DWT_CTRL.CYCCNTENA, bit 0: starts the cycle counter (the same
/// manual, DWT_CTRL; the cortex-m crate's `CYCCNTENA`).
pub(crate) const DWT_CTRL_CYCCNTENA: u32 = 1;
