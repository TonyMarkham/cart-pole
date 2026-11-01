//! The board's WiFi, without polling. The ESP32-S3 runs the user's interrupt
//! bridge (`D:\git\unor4-wifi-interrupt-bridge`, branch `feature/interrupt`,
//! `docs/releases/v0.6.0-interrupt.2.md`), reached over SCI1, P501/P502,
//! board-internal. With push mode on, the bridge sends every datagram for the
//! socket to the processor unasked:
//!
//! `+UDPRX: <sock>,<len>|<len bytes>\r\n`
//!
//! Each byte raises the UART's receive interrupt, so the processor sleeps
//! until one arrives, as the bridge's `examples/UDPPushSleep` does. Push mode
//! covers UDP only, so the request and its answer are datagrams, not HTTP.
//!
//! The join is the sequence arduino-embedded-rust's `wifi-sample` runs, proven
//! on this board 2026-10-21 (its WiFi.md has every source). The answer's
//! commands are the bridge's `cmds_wifi_udp.h`:
//!
//! - `AT+UDPBEGIN=<port>` → `+UDPBEGIN: <sock>`.
//! - `AT+UDPPUSH=<sock>,1` → `OK`; `ERROR` on a bridge without push mode.
//! - `AT+UDPREMOTEIP=<sock>` and `AT+UDPREMOTEPORT=<sock>` → the sender of
//!   the last datagram.
//! - `AT+UDPBEGINPACKETIP=<sock>,<port>,<ip>`, then
//!   `AT+UDPWRITE=<sock>,<n>` and exactly n bytes, then
//!   `AT+UDPENDPACKET=<sock>`: one datagram out, each answered `OK`.

use core::fmt::Write;

use embedded_hal::delay::DelayNs;
use unor4_hal::{delay::Delay, uart::UartIrq};

use crate::{
    buffer::Buffer,
    pins::Pins,
    settings::{AT_BEGINSTA, AT_HOSTNAME, AT_UDPBEGIN},
};

/// The one request answered, the whole datagram: `PINS_REQUEST` in the
/// Orin's webserver. Anything else is ignored, so nothing sent here can
/// move the cart.
const PINS_REQUEST: &[u8] = b"pins";

/// A frame's start, and the end of its header.
const FRAME_PROMPT: &[u8] = b"+UDPRX: ";
const FRAME_HEADER_END: u8 = b'|';

/// The start of a request kept; longer than `PINS_REQUEST` so that a longer
/// datagram does not pass for it.
const REQUEST_MAX: usize = 16;

/// Buffer for one AT reply.
const RESP_MAX: usize = 256;

/// Wait between looks at the receive ring while a command's reply is due,
/// µs. The ring holds what arrives meanwhile, so this only sets how finely
/// the reply's timeout is counted.
const POLL_US: u32 = 100;

/// How long a frame's payload may take once its header has arrived, ms: far
/// longer than the few bytes take at 115,200 baud.
const FRAME_TIMEOUT_MS: u32 = 100;

/// "Connected": the bridge's `WIFI_ST_CONNECTED` (at_handler.h).
const WL_CONNECTED: u32 = 3;

/// Why the join failed.
#[derive(Clone, Copy, Debug)]
pub enum JoinError {
    /// The bridge never answered `AT`.
    NoBridge,
    /// `+WIFIMODE` or `+BEGINSTA` answered ERROR.
    Rejected,
    /// The join never reached "connected"; the last status seen, if any:
    /// 1 = network not found, 4 = connect failed (usually the password).
    NotConnected(Option<u32>),
    /// Joined, but no address arrived from the router.
    NoAddress,
    /// `+UDPBEGIN` answered ERROR.
    NoSocket,
    /// `+UDPPUSH` answered ERROR: the bridge is not the interrupt fork.
    NoPushMode,
}

impl JoinError {
    pub fn describe(&self) -> &'static str {
        match self {
            JoinError::NoBridge => "the ESP32-S3 bridge never answered AT",
            JoinError::Rejected => "the bridge refused station mode or the join",
            JoinError::NotConnected(Some(1)) => "network not found (out of range, or 5 GHz only: the radio is 2.4 GHz)",
            JoinError::NotConnected(Some(4)) => "connect failed: usually a wrong password",
            JoinError::NotConnected(_) => "the join did not finish within 10 s",
            JoinError::NoAddress => "joined, but the router gave no address",
            JoinError::NoSocket => "the bridge refused to open the UDP socket",
            JoinError::NoPushMode => "this bridge has no push mode: flash the interrupt bridge (its docs/flash-interrupt-procedure.md)",
        }
    }
}

/// Short text from an AT reply: an address, a port or a version.
pub struct Text {
    bytes: [u8; 15],
    len: usize,
}

impl Text {
    fn new(text: &[u8]) -> Option<Self> {
        if text.is_empty() || text.len() > 15 {
            return None;
        }
        let mut bytes = [0; 15];
        bytes[..text.len()].copy_from_slice(text);
        Some(Self { bytes, len: text.len() })
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("?")
    }
}

/// What the join found, for the serial port.
pub struct Joined {
    pub address: Text,
    pub bridge_version: Option<Text>,
}

pub struct Wifi {
    at: &'static UartIrq<ra4m1::SCI1>,
    resp: [u8; RESP_MAX],
    socket: i32,
    /// The latest datagram's start, and whether it is still to be answered.
    request: [u8; REQUEST_MAX],
    request_len: usize,
    request_waiting: bool,
}

impl Wifi {
    /// Joins the network, opens the UDP socket and turns push mode on.
    /// `at` must be attached, with interrupts enabled.
    pub fn join(at: &'static UartIrq<ra4m1::SCI1>, delay: &mut Delay) -> Result<(Self, Joined), JoinError> {
        let mut wifi = Self {
            at,
            resp: [0; RESP_MAX],
            socket: -1,
            request: [0; REQUEST_MAX],
            request_len: 0,
            request_waiting: false,
        };

        // Handshake: any reply means the AT server is up; the ESP32 may
        // still be booting, so retry.
        let mut alive = false;
        for _ in 0..10 {
            if wifi.command(delay, b"AT\r\n", 700).is_some() {
                alive = true;
                break;
            }
            delay.delay_ms(300);
        }
        if !alive {
            return Err(JoinError::NoBridge);
        }

        // Clears WiFi state kept from an earlier sketch, push mode included,
        // which survives a reset of the processor (the release notes).
        // Best-effort, as in WiFiS3's ModemClass::begin: 3 tries, the result
        // gates nothing.
        for _ in 0..3 {
            if let Some((_, true)) = wifi.command(delay, b"AT+SOFTRESETWIFI\r\n", 500) {
                break;
            }
        }

        let bridge_version = match wifi.command(delay, b"AT+FWVERSION?\r\n", 1_000) {
            Some((n, true)) => after(&wifi.resp[..n], b"+FWVERSION: ").and_then(Text::new),
            _ => None,
        };

        // The hostname takes effect when the mode changes into station
        // mode, so drop to mode 0 first. Both best-effort.
        wifi.command(delay, b"AT+WIFIMODE=0\r\n", 2_000);
        wifi.command(delay, AT_HOSTNAME.as_bytes(), 1_000);
        if !matches!(wifi.command(delay, b"AT+WIFIMODE=1\r\n", 2_000), Some((_, true))) {
            return Err(JoinError::Rejected);
        }
        if !matches!(wifi.command(delay, AT_BEGINSTA.as_bytes(), 3_000), Some((_, true))) {
            return Err(JoinError::Rejected);
        }

        // The bridge joins in the background: ask for up to 10 s.
        let mut status = None;
        for _ in 0..20 {
            delay.delay_ms(500);
            if let Some((n, true)) = wifi.command(delay, b"AT+GETSTATUS?\r\n", 1_000) {
                status = after(&wifi.resp[..n], b"+GETSTATUS: ").and_then(parse_u32);
                if status == Some(WL_CONNECTED) {
                    break;
                }
            }
        }
        if status != Some(WL_CONNECTED) {
            return Err(JoinError::NotConnected(status));
        }

        // "Connected" comes before the router's lease: the address reads
        // 0.0.0.0 until then. `+IPSTA=0` reads the station's address.
        let mut address = None;
        for _ in 0..50 {
            if let Some((n, true)) = wifi.command(delay, b"AT+IPSTA=0\r\n", 1_000)
                && let Some(text) = after(&wifi.resp[..n], b"+IPSTA: ")
                && text.iter().any(|&byte| byte != b'0' && byte != b'.')
                && let Some(text) = Text::new(text)
            {
                address = Some(text);
                break;
            }
            delay.delay_ms(200);
        }
        let address = address.ok_or(JoinError::NoAddress)?;

        wifi.socket = match wifi.command(delay, AT_UDPBEGIN.as_bytes(), 1_000) {
            Some((n, true)) => after(&wifi.resp[..n], b"+UDPBEGIN: ").and_then(parse_i32).ok_or(JoinError::NoSocket)?,
            _ => return Err(JoinError::NoSocket),
        };
        let mut push = Buffer::<32>::new();
        write!(push, "AT+UDPPUSH={},1\r\n", wifi.socket).map_err(|_| JoinError::NoPushMode)?;
        if !matches!(wifi.command(delay, push.as_bytes(), 1_000), Some((_, true))) {
            return Err(JoinError::NoPushMode);
        }
        Ok((wifi, Joined { address, bridge_version }))
    }

    /// Sleeps until a datagram has arrived, then answers it if it asks for
    /// the levels. Only the UART's interrupt wakes the processor.
    pub fn serve(&mut self, delay: &mut Delay, pins: &Pins) {
        if !self.request_waiting {
            self.sleep_until_frame(delay);
        }
        self.request_waiting = false;
        if &self.request[..self.request_len] == PINS_REQUEST {
            self.answer(delay, pins);
        }
    }

    /// Sleeps while the receive ring is empty, and returns once a frame has
    /// been read. A line that is not a frame is not expected once push mode
    /// is on, and is dropped.
    fn sleep_until_frame(&mut self, delay: &mut Delay) {
        let mut n = 0;
        while !self.request_waiting {
            // Masked across the check and the sleep, so a byte arriving
            // between them cannot be missed: a pending interrupt still ends
            // WFI, and its handler runs when interrupts are enabled again
            // (examples/UDPPushSleep, loop()).
            cortex_m::interrupt::disable();
            let byte = self.at.read_byte();
            if byte.is_none() {
                cortex_m::asm::wfi();
            }
            // Safety: nothing here is in a critical section, and every
            // handler linked is the UART's, which keeps to its own state.
            unsafe { cortex_m::interrupt::enable() };

            let Some(byte) = byte else {
                continue;
            };
            if n < RESP_MAX {
                self.resp[n] = byte;
                n += 1;
            }
            if byte == b'\n' || (byte == FRAME_HEADER_END && self.read_frame(delay, n).is_some()) {
                n = 0;
            }
        }
    }

    /// One datagram out: the levels, to whoever sent the last datagram. Any
    /// step's failure drops the answer, and the Orin's wait for it times out.
    /// A frame arriving meanwhile is kept by `reply`, for the next `serve`.
    fn answer(&mut self, delay: &mut Delay, pins: &Pins) -> Option<()> {
        let socket = self.socket;

        let mut cmd = Buffer::<64>::new();
        write!(cmd, "AT+UDPREMOTEIP={socket}\r\n").ok()?;
        let (n, _) = self.ask(delay, cmd.as_bytes()).filter(|&(_, ok)| ok)?;
        let address = after(&self.resp[..n], b"+UDPREMOTEIP: ").and_then(Text::new)?;

        let mut cmd = Buffer::<64>::new();
        write!(cmd, "AT+UDPREMOTEPORT={socket}\r\n").ok()?;
        let (n, _) = self.ask(delay, cmd.as_bytes()).filter(|&(_, ok)| ok)?;
        let port = after(&self.resp[..n], b"+UDPREMOTEPORT: ").and_then(Text::new)?;

        let mut cmd = Buffer::<64>::new();
        write!(cmd, "AT+UDPBEGINPACKETIP={socket},{},{}\r\n", port.as_str(), address.as_str()).ok()?;
        self.ask(delay, cmd.as_bytes()).filter(|&(_, ok)| ok)?;

        let mut levels = Buffer::<64>::new();
        pins.write_levels(&mut levels).ok()?;
        let mut cmd = Buffer::<64>::new();
        write!(cmd, "AT+UDPWRITE={socket},{}\r\n", levels.as_bytes().len()).ok()?;
        self.at.write_all(cmd.as_bytes());
        self.at.write_all(levels.as_bytes());
        self.reply(delay, 1_000).filter(|&(_, ok)| ok)?;

        let mut cmd = Buffer::<64>::new();
        write!(cmd, "AT+UDPENDPACKET={socket}\r\n").ok()?;
        self.ask(delay, cmd.as_bytes()).filter(|&(_, ok)| ok)?;
        Some(())
    }

    /// Before push mode: discards stale bytes, then sends one command.
    fn command(&mut self, delay: &mut Delay, cmd: &[u8], timeout_ms: u32) -> Option<(usize, bool)> {
        self.drain(delay);
        self.at.write_all(cmd);
        self.reply(delay, timeout_ms)
    }

    /// With push mode on: sends one command with no drain, which would
    /// throw away a frame.
    fn ask(&mut self, delay: &mut Delay, cmd: &[u8]) -> Option<(usize, bool)> {
        self.at.write_all(cmd);
        self.reply(delay, 1_000)
    }

    /// Collects a reply until `OK\r\n` or `ERROR\r\n`: `(length, ok)`, or
    /// None if the line stays idle for `timeout_ms`. A frame can arrive
    /// between a command and its reply (the release notes), so it is lifted
    /// out as soon as its header ends.
    fn reply(&mut self, delay: &mut Delay, timeout_ms: u32) -> Option<(usize, bool)> {
        let timeout_us = timeout_ms.saturating_mul(1_000);
        let mut n = 0;
        let mut waited_us = 0;
        while waited_us < timeout_us {
            let Some(byte) = self.at.read_byte() else {
                delay.delay_us(POLL_US);
                waited_us += POLL_US;
                continue;
            };
            if n < RESP_MAX {
                self.resp[n] = byte;
                n += 1;
            }
            if byte == FRAME_HEADER_END
                && let Some(start) = self.read_frame(delay, n)
            {
                n = start;
                continue;
            }
            if ends_with(&self.resp[..n], b"OK\r\n") {
                return Some((n, true));
            }
            if ends_with(&self.resp[..n], b"ERROR\r\n") {
                return Some((n, false));
            }
        }
        None
    }

    /// If the first `n` bytes of `resp` end in a frame's header, reads the
    /// frame's payload and line end, keeps the payload's start as the waiting
    /// request, and returns where the header began. The payload is counted,
    /// not scanned, so a datagram holding `|` or `OK` cannot end it early.
    fn read_frame(&mut self, delay: &mut Delay, n: usize) -> Option<usize> {
        let start = self.resp[..n].windows(FRAME_PROMPT.len()).position(|window| window == FRAME_PROMPT)?;
        let header = &self.resp[start + FRAME_PROMPT.len()..n - 1];
        let comma = header.iter().position(|&byte| byte == b',')?;
        let len = parse_u32(&header[comma + 1..])? as usize;

        let mut kept = 0;
        for read in 0..len + 2 {
            let byte = self.frame_byte(delay)?;
            if read < len && kept < REQUEST_MAX {
                self.request[kept] = byte;
                kept += 1;
            }
        }
        self.request_len = kept;
        self.request_waiting = true;
        Some(start)
    }

    /// The next byte of a frame whose header has arrived.
    fn frame_byte(&mut self, delay: &mut Delay) -> Option<u8> {
        let mut waited_us = 0;
        while waited_us < FRAME_TIMEOUT_MS * 1_000 {
            if let Some(byte) = self.at.read_byte() {
                return Some(byte);
            }
            delay.delay_us(POLL_US);
            waited_us += POLL_US;
        }
        None
    }

    /// Discards stale bytes until the line has been quiet for 20 ms.
    fn drain(&mut self, delay: &mut Delay) {
        let mut quiet_us = 0;
        while quiet_us < 20_000 {
            if self.at.read_byte().is_some() {
                quiet_us = 0;
            } else {
                delay.delay_us(POLL_US);
                quiet_us += POLL_US;
            }
        }
    }
}

fn ends_with(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.len() >= needle.len() && &bytes[bytes.len() - needle.len()..] == needle
}

/// The value after `prompt`, up to the line end.
fn after<'a>(reply: &'a [u8], prompt: &[u8]) -> Option<&'a [u8]> {
    let start = reply.windows(prompt.len()).position(|window| window == prompt)? + prompt.len();
    let rest = &reply[start..];
    let stop = rest.iter().position(|&byte| byte == b'\r').unwrap_or(rest.len());
    Some(&rest[..stop])
}

fn parse_u32(text: &[u8]) -> Option<u32> {
    core::str::from_utf8(text).ok()?.trim().parse().ok()
}

fn parse_i32(text: &[u8]) -> Option<i32> {
    core::str::from_utf8(text).ok()?.trim().parse().ok()
}
