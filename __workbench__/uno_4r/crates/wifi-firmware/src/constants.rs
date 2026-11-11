//! Fixed facts: the bench firmware's WiFi and USB constants, and this
//! firmware's USB protocol. Every protocol word must match the PC's copy in
//! `__workbench__/pc/crates/wifi-host/src/protocol.rs`.

/// What the board sends down USB at every start.
pub(crate) const LINE_READY: &[u8] = b"READY\n";

/// The end of every line on the USB line, both ways.
pub(crate) const LINE_END: &[u8] = b"\n";

/// The longest line the board takes over USB, bytes: a `SYNC` request is 31.
pub(crate) const LINE_MAX: usize = 32;

/// The PC's question: is the board running, and where can it be reached?
pub(crate) const LINE_STATUS: &[u8] = b"STATUS";

/// The answers to `STATUS`: `OK STATUS joining`, `OK STATUS running <address>
/// <port>`, or `OK STATUS stopped <why>`.
pub(crate) const REPLY_STATUS: &[u8] = b"OK STATUS ";
pub(crate) const STATUS_JOINING: &[u8] = b"joining";
pub(crate) const STATUS_RUNNING: &[u8] = b"running ";

/// A clock sync, both ways, 31 characters and the line end:
/// `SYNC <n> <t1> <t2>`, each field 8 hex digits. The PC sends `t1` and `t2`
/// as zeros; the board answers with the same `n` and its two stamps: `t1`
/// when the request's line ended, `t2` just before the answer's first byte.
/// Request and answer are the same length, so each takes the same time on
/// the 115,200-baud line.
pub(crate) const SYNC_PREFIX: &[u8] = b"SYNC ";
pub(crate) const SYNC_LINE_LEN: usize = 31;

/// A datagram's report: `PING <payload> <t>`, the payload as 6 hex digits and
/// the board's stamp of when it had the datagram whole as 8.
pub(crate) const PING_PREFIX: &[u8] = b"PING ";

/// What separates a line's fields.
pub(crate) const FIELD_SEPARATOR: u8 = b' ';

/// Hex digits per stamp or sequence number (a u32), and per payload byte.
pub(crate) const U32_HEX_DIGITS: usize = 8;
pub(crate) const BYTE_HEX_DIGITS: usize = 2;

/// The datagrams the board stamps: exactly a command's size, so the WiFi
/// chip's serial hand-off takes as long as a command's (__documentation__/real-loop-dia.md,
/// *Contracts*: a command is 3 bytes).
pub(crate) const PING_SIZE: u32 = 3;

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

/// Hex, the base every number on the USB line is written in: 16 values a
/// digit, 4 bits each, so a digit is a value's bits masked by 0xF after a
/// shift of 4 per place.
pub(crate) const HEX_RADIX: u32 = 16;
pub(crate) const BITS_PER_HEX_DIGIT: u32 = 4;
pub(crate) const HEX_DIGIT_MASK: u32 = 0xF;
