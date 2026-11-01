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

use crate::{
    buffer::Buffer,
    constants::{
        AT_COMMAND_MAX, CENTRE_REQUEST, FRAME_HEADER_END, FRAME_PROMPT, IDLER_REQUEST,
        MOTOR_REQUEST, PINS_ANSWER_MAX, PINS_REQUEST, REQUEST_MAX, RESP_MAX, SEEK_ANSWER_MAX,
        WL_CONNECT_FAILED, WL_CONNECTED, WL_NETWORK_NOT_FOUND,
    },
    error::{AppError, result::AppResult},
    joined::Joined,
    moved::Moved,
    pins::Pins,
    rail::Rail,
    settings::{
        ADDRESS_PAUSE_MS, ADDRESS_TRIES, AT_BEGINSTA, AT_HOSTNAME, AT_UDPBEGIN,
        BEGIN_STA_TIMEOUT_MS, DRAIN_QUIET_MS, FRAME_TIMEOUT_MS, HANDSHAKE_PAUSE_MS,
        HANDSHAKE_TIMEOUT_MS, HANDSHAKE_TRIES, IDLER_END, JOIN_TIMED_OUT, MOTOR_END, POLL_US,
        REPLY_TIMEOUT_MS, SOFT_RESET_TIMEOUT_MS, SOFT_RESET_TRIES, STATUS_PAUSE_MS, STATUS_TRIES,
        WIFI_MODE_TIMEOUT_MS,
    },
    text::Text,
};

use core::fmt::Write;
use embedded_hal::delay::DelayNs;
use unor4_hal::{delay::Delay, uart::UartIrq};

pub struct Wifi {
    at: &'static UartIrq<ra4m1::SCI1>,
    resp: [u8; RESP_MAX],
    socket: i32,
    /// The latest datagram's start, and whether it is still to be answered.
    request: [u8; REQUEST_MAX],
    request_len: usize,
    request_waiting: bool,
    rail: Rail,
}

impl Wifi {
    /// Joins the network, opens the UDP socket and turns push mode on.
    /// `at` must be attached, with interrupts enabled.
    pub fn join(at: &'static UartIrq<ra4m1::SCI1>, delay: &mut Delay) -> AppResult<(Self, Joined)> {
        let mut wifi = Self {
            at,
            resp: [0; RESP_MAX],
            socket: -1,
            request: [0; REQUEST_MAX],
            request_len: 0,
            request_waiting: false,
            rail: Rail::new(),
        };

        // Handshake: any reply means the AT server is up; the ESP32 may
        // still be booting, so retry.
        let mut alive = false;
        for _ in 0..HANDSHAKE_TRIES {
            if wifi
                .command(delay, b"AT\r\n", HANDSHAKE_TIMEOUT_MS)
                .is_some()
            {
                alive = true;
                break;
            }
            delay.delay_ms(HANDSHAKE_PAUSE_MS);
        }
        if !alive {
            return Err(AppError::bridge("the ESP32-S3 bridge never answered AT"));
        }

        // Clears WiFi state kept from an earlier sketch, push mode included,
        // which survives a reset of the processor (the release notes).
        // Best-effort, as in WiFiS3's ModemClass::begin: the result gates
        // nothing.
        for _ in 0..SOFT_RESET_TRIES {
            if let Some((_, true)) =
                wifi.command(delay, b"AT+SOFTRESETWIFI\r\n", SOFT_RESET_TIMEOUT_MS)
            {
                break;
            }
        }

        let bridge_version = match wifi.command(delay, b"AT+FWVERSION?\r\n", REPLY_TIMEOUT_MS) {
            Some((n, true)) => after(&wifi.resp[..n], b"+FWVERSION: ").and_then(Text::new),
            _ => None,
        };

        // The hostname takes effect when the mode changes into station
        // mode, so drop to mode 0 first. Both best-effort.
        wifi.command(delay, b"AT+WIFIMODE=0\r\n", WIFI_MODE_TIMEOUT_MS);
        wifi.command(delay, AT_HOSTNAME.as_bytes(), REPLY_TIMEOUT_MS);
        if !matches!(
            wifi.command(delay, b"AT+WIFIMODE=1\r\n", WIFI_MODE_TIMEOUT_MS),
            Some((_, true))
        ) {
            return Err(AppError::bridge("the bridge refused station mode"));
        }
        if !matches!(
            wifi.command(delay, AT_BEGINSTA.as_bytes(), BEGIN_STA_TIMEOUT_MS),
            Some((_, true))
        ) {
            return Err(AppError::bridge("the bridge refused the join"));
        }

        // The bridge joins in the background, so keep asking.
        let mut status = None;
        for _ in 0..STATUS_TRIES {
            delay.delay_ms(STATUS_PAUSE_MS);
            if let Some((n, true)) = wifi.command(delay, b"AT+GETSTATUS?\r\n", REPLY_TIMEOUT_MS) {
                status = after(&wifi.resp[..n], b"+GETSTATUS: ").and_then(parse_u32);
                if status == Some(WL_CONNECTED) {
                    break;
                }
            }
        }
        if status != Some(WL_CONNECTED) {
            return Err(AppError::join(match status {
                Some(WL_NETWORK_NOT_FOUND) => {
                    "network not found (out of range, or 5 GHz only: the radio is 2.4 GHz)"
                }
                Some(WL_CONNECT_FAILED) => "connect failed: usually a wrong password",
                _ => JOIN_TIMED_OUT,
            }));
        }

        // "Connected" comes before the router's lease: the address reads
        // 0.0.0.0 until then. `+IPSTA=0` reads the station's address.
        let mut address = None;
        for _ in 0..ADDRESS_TRIES {
            if let Some((n, true)) = wifi.command(delay, b"AT+IPSTA=0\r\n", REPLY_TIMEOUT_MS)
                && let Some(text) = after(&wifi.resp[..n], b"+IPSTA: ")
                && text.iter().any(|&byte| byte != b'0' && byte != b'.')
                && let Some(text) = Text::new(text)
            {
                address = Some(text);
                break;
            }
            delay.delay_ms(ADDRESS_PAUSE_MS);
        }
        let address =
            address.ok_or_else(|| AppError::join("joined, but the router gave no address"))?;

        wifi.socket = match wifi.command(delay, AT_UDPBEGIN.as_bytes(), REPLY_TIMEOUT_MS) {
            Some((n, true)) => after(&wifi.resp[..n], b"+UDPBEGIN: ")
                .and_then(parse_i32)
                .ok_or_else(|| AppError::bridge("the bridge refused to open the UDP socket"))?,
            _ => {
                return Err(AppError::bridge(
                    "the bridge refused to open the UDP socket",
                ));
            }
        };
        let mut push = Buffer::<AT_COMMAND_MAX>::new();
        write!(push, "AT+UDPPUSH={},1\r\n", wifi.socket)?;
        if !matches!(
            wifi.command(delay, push.as_bytes(), REPLY_TIMEOUT_MS),
            Some((_, true))
        ) {
            return Err(AppError::bridge(
                "this bridge has no push mode: flash the interrupt bridge (its docs/flash-interrupt-procedure.md)",
            ));
        }
        Ok((
            wifi,
            Joined {
                address,
                bridge_version,
            },
        ))
    }

    /// Sleeps until a datagram has arrived, then does what it asks. Only the
    /// UART's interrupt wakes the processor.
    pub fn serve(&mut self, delay: &mut Delay, pins: &mut Pins) -> AppResult<()> {
        if !self.request_waiting {
            self.sleep_until_frame(delay);
        }
        self.request_waiting = false;
        if self.asked(PINS_REQUEST) {
            self.answer_levels(delay, pins);
            Ok(())
        } else if self.asked(IDLER_REQUEST) {
            let at = self.at;
            self.travel(delay, pins, |rail, pins, delay| {
                rail.seek(at, pins, delay, IDLER_END)
            })
        } else if self.asked(MOTOR_REQUEST) {
            let at = self.at;
            self.travel(delay, pins, |rail, pins, delay| {
                rail.seek(at, pins, delay, MOTOR_END)
            })
        } else if let Some(rest) = self.request[..self.request_len].strip_prefix(CENTRE_REQUEST) {
            let speed = centre_speed(rest);
            let at = self.at;
            self.travel(delay, pins, |rail, pins, delay| {
                rail.centre(at, pins, delay, speed)
            })
        } else {
            Ok(())
        }
    }

    fn asked(&self, request: &[u8]) -> bool {
        &self.request[..self.request_len] == request
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

    /// The levels, to whoever asked.
    fn answer_levels(&mut self, delay: &mut Delay, pins: &Pins) -> Option<()> {
        let (address, port) = self.sender(delay)?;
        let mut levels = Buffer::<PINS_ANSWER_MAX>::new();
        pins.write_levels(&mut levels).ok()?;
        self.send(delay, &address, &port, levels.as_bytes())
    }

    /// Makes one move, then tells whoever asked how it ended. The
    /// sender is read first, because no one being told is a reason not to
    /// move. The pulses leave time only to listen for `Stop`, so every other
    /// datagram that arrives during the move is thrown away: a request left over
    /// from then must not start another move.
    fn travel(
        &mut self,
        delay: &mut Delay,
        pins: &mut Pins,
        make_move: impl FnOnce(&mut Rail, &mut Pins, &mut Delay) -> AppResult<Moved>,
    ) -> AppResult<()> {
        let Some((address, port)) = self.sender(delay) else {
            return Ok(());
        };
        let moved = make_move(&mut self.rail, pins, delay);
        self.drain(delay);
        self.request_waiting = false;

        let mut answer = Buffer::<SEEK_ANSWER_MAX>::new();
        match &moved {
            Ok(moved) => moved.write_answer(&mut answer)?,
            Err(error) => error.write_answer(&mut answer)?,
        }
        self.send(delay, &address, &port, answer.as_bytes());
        moved.map(|_| ())
    }

    /// The address and port of whoever sent the last datagram.
    fn sender(&mut self, delay: &mut Delay) -> Option<(Text, Text)> {
        let socket = self.socket;

        let mut cmd = Buffer::<AT_COMMAND_MAX>::new();
        write!(cmd, "AT+UDPREMOTEIP={socket}\r\n").ok()?;
        let (n, _) = self.ask(delay, cmd.as_bytes()).filter(|&(_, ok)| ok)?;
        let address = after(&self.resp[..n], b"+UDPREMOTEIP: ").and_then(Text::new)?;

        let mut cmd = Buffer::<AT_COMMAND_MAX>::new();
        write!(cmd, "AT+UDPREMOTEPORT={socket}\r\n").ok()?;
        let (n, _) = self.ask(delay, cmd.as_bytes()).filter(|&(_, ok)| ok)?;
        let port = after(&self.resp[..n], b"+UDPREMOTEPORT: ").and_then(Text::new)?;

        Some((address, port))
    }

    /// One datagram out. Any step's failure drops it, and the Orin's wait for
    /// it times out. A frame arriving meanwhile is kept by `reply`, for the
    /// next `serve`.
    fn send(
        &mut self,
        delay: &mut Delay,
        address: &Text,
        port: &Text,
        payload: &[u8],
    ) -> Option<()> {
        let socket = self.socket;

        let mut cmd = Buffer::<AT_COMMAND_MAX>::new();
        write!(
            cmd,
            "AT+UDPBEGINPACKETIP={socket},{},{}\r\n",
            port.as_str(),
            address.as_str()
        )
        .ok()?;
        self.ask(delay, cmd.as_bytes()).filter(|&(_, ok)| ok)?;

        let mut cmd = Buffer::<AT_COMMAND_MAX>::new();
        write!(cmd, "AT+UDPWRITE={socket},{}\r\n", payload.len()).ok()?;
        self.at.write_all(cmd.as_bytes());
        self.at.write_all(payload);
        self.reply(delay, REPLY_TIMEOUT_MS).filter(|&(_, ok)| ok)?;

        let mut cmd = Buffer::<AT_COMMAND_MAX>::new();
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
        self.reply(delay, REPLY_TIMEOUT_MS)
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
        let start = self.resp[..n]
            .windows(FRAME_PROMPT.len())
            .position(|window| window == FRAME_PROMPT)?;
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
        while waited_us < FRAME_TIMEOUT_MS.saturating_mul(1_000) {
            if let Some(byte) = self.at.read_byte() {
                return Some(byte);
            }
            delay.delay_us(POLL_US);
            waited_us += POLL_US;
        }
        None
    }

    /// Discards stale bytes until the line has been quiet for `drain_quiet_ms`.
    fn drain(&mut self, delay: &mut Delay) {
        let mut quiet_us = 0;
        while quiet_us < DRAIN_QUIET_MS.saturating_mul(1_000) {
            if self.at.read_byte().is_some() {
                quiet_us = 0;
            } else {
                delay.delay_us(POLL_US);
                quiet_us += POLL_US;
            }
        }
    }
}

/// The speed after `MoveToCentre`: None with nothing after it. Anything that is
/// not a space and a whole number becomes 0, which `Rail::centre` answers
/// bad_speed, so a mistyped speed never moves the cart.
fn centre_speed(rest: &[u8]) -> Option<u32> {
    if rest.is_empty() {
        return None;
    }
    Some(rest.strip_prefix(b" ").and_then(parse_u32).unwrap_or(0))
}

fn ends_with(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.len() >= needle.len() && &bytes[bytes.len() - needle.len()..] == needle
}

/// The value after `prompt`, up to the line end.
fn after<'a>(reply: &'a [u8], prompt: &[u8]) -> Option<&'a [u8]> {
    let start = reply
        .windows(prompt.len())
        .position(|window| window == prompt)?
        + prompt.len();
    let rest = &reply[start..];
    let stop = rest
        .iter()
        .position(|&byte| byte == b'\r')
        .unwrap_or(rest.len());
    Some(&rest[..stop])
}

fn parse_u32(text: &[u8]) -> Option<u32> {
    core::str::from_utf8(text).ok()?.trim().parse().ok()
}

fn parse_i32(text: &[u8]) -> Option<i32> {
    core::str::from_utf8(text).ok()?.trim().parse().ok()
}
