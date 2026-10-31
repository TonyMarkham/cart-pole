//! The board's WiFi: the ESP32-S3's AT server (the stock UNO R4 WiFi
//! bridge firmware) on SCI1, P501/P502, board-internal. The join is the
//! sequence arduino-embedded-rust's `wifi-sample` runs, proven on this
//! board 2026-10-21 (its WiFi.md has every source). The pin-state page
//! is the bridge's TCP server (uno-r4-wifi-usb-bridge 0.6.0,
//! `cmds_wifi_netif.h`):
//!
//! - `AT+BEGINSERVER=<port>` → `+BEGINSERVER: <server>`.
//! - `AT+SERVERAVAILABLE=<server>` → `+SERVERAVAILABLE: <socket>`, a
//!   connected client that has sent something, or -1.
//! - `AT+CLIENTRECEIVE=<socket>,<max>` → `+CLIENTRECEIVE: <n>|` then
//!   exactly n bytes, then `OK`.
//! - `AT+CLIENTSEND=<socket>,<n>` then exactly n bytes → `OK`.
//! - `AT+CLIENTCLOSE=<socket>` → `OK`.
//!
//! The bridge speaks only when asked, so the link is silent unless the
//! program asks. Requests wait in the ESP32 until it does: when idle,
//! and every `stop_check_ms` during a slow move, to catch STOP. A button's
//! answer is held until its action has finished, so the browser waits
//! through a move and sends nothing more; `+SERVERAVAILABLE` offers only
//! a client with unread data (`cmds_wifi_netif.h`, `_SERVERAVAILABLE`:
//! `client.available() > 0`), so the held one does not come back and a
//! STOP from the same page still gets through.

use core::fmt::Write;

use embedded_hal::delay::DelayNs;
use unor4_hal::{
    delay::Delay,
    uart::{ReadError, Uart},
};

use crate::{
    action::Action,
    buffer::Buffer,
    experiment::ExperimentReport,
    page,
    pins::PinStates,
    report::Report,
    settings::{AT_BEGINSERVER, AT_BEGINSTA, AT_HOSTNAME, REPORT_HOST, REPORT_PORT},
};

/// The answer to a button's POST: back to the page, so a reload repeats
/// the GET, never the POST (RFC 9110, 15.4.4).
const SEE_OTHER: &[u8] = b"HTTP/1.0 303 See Other\r\nLocation: /\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

const HTML_HEADER: &str = "HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n";

/// The start of a request kept for its first line, `POST /stop HTTP/1.1`
/// and the like; the longest path is 10 bytes.
const HEAD_MAX: usize = 48;

/// Buffer for one AT reply.
const RESP_MAX: usize = 256;

/// Poll interval while the receiver is empty, µs. One byte at 115,200
/// baud takes ~87 µs and the SCI holds one received byte, so a longer
/// wait loses bytes (WiFi.md: 1 ms failed, 10 µs joined, 2026-10-21).
const POLL_US: u32 = 10;

/// "Connected": the bridge's `WIFI_ST_CONNECTED` (at_handler.h).
const WL_CONNECTED: u32 = 3;

/// The request bytes read per `+CLIENTRECEIVE`, and the most rounds of
/// it before answering: a request is read whole so the socket closes
/// cleanly; only its first line is kept.
const RECEIVE_CHUNK: u32 = 128;
const RECEIVE_ROUNDS: u32 = 8;

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
    /// `+BEGINSERVER` answered ERROR.
    NoServer,
}

impl JoinError {
    pub fn describe(&self) -> &'static str {
        match self {
            JoinError::NoBridge => "the ESP32-S3 bridge never answered AT",
            JoinError::Rejected => "the bridge refused station mode or the join",
            JoinError::NotConnected(Some(1)) => {
                "network not found (out of range, or 5 GHz only: the radio is 2.4 GHz)"
            }
            JoinError::NotConnected(Some(4)) => "connect failed: usually a wrong password",
            JoinError::NotConnected(_) => "the join did not finish within 10 s",
            JoinError::NoAddress => "joined, but the router gave no address",
            JoinError::NoServer => "the bridge refused to open the TCP server",
        }
    }
}

/// The board's IPv4 address as text.
pub struct Address {
    bytes: [u8; 15],
    len: usize,
}

impl Address {
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("?")
    }
}

pub struct Wifi {
    uart: Uart<ra4m1::SCI1>,
    resp: [u8; RESP_MAX],
    server: i32,
    /// The start of the request being read.
    head: [u8; HEAD_MAX],
    head_len: usize,
    /// The socket of a button whose action is running, answered by
    /// `finish` once it ends.
    held: Option<i32>,
}

impl Wifi {
    /// Joins the network and opens the TCP server.
    pub fn join(uart: Uart<ra4m1::SCI1>, delay: &mut Delay) -> Result<(Self, Address), JoinError> {
        let mut wifi = Self {
            uart,
            resp: [0; RESP_MAX],
            server: -1,
            head: [0; HEAD_MAX],
            head_len: 0,
            held: None,
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

        // Clear WiFi state kept from an earlier sketch. Best-effort, as in
        // WiFiS3's ModemClass::begin: 3 tries, the result gates nothing.
        for _ in 0..3 {
            if let Some((_, true)) = wifi.command(delay, b"AT+SOFTRESETWIFI\r\n", 500) {
                break;
            }
        }

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

        // The bridge joins in the background: poll for up to 10 s.
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
                && !text.is_empty()
                && text.len() <= 15
                && text.iter().any(|&byte| byte != b'0' && byte != b'.')
            {
                let mut bytes = [0; 15];
                bytes[..text.len()].copy_from_slice(text);
                address = Some(Address { bytes, len: text.len() });
                break;
            }
            delay.delay_ms(200);
        }
        let address = address.ok_or(JoinError::NoAddress)?;

        match wifi.command(delay, AT_BEGINSERVER.as_bytes(), 1_000) {
            Some((n, true)) => {
                wifi.server = after(&wifi.resp[..n], b"+BEGINSERVER: ")
                    .and_then(parse_i32)
                    .ok_or(JoinError::NoServer)?;
            }
            _ => return Err(JoinError::NoServer),
        }
        Ok((wifi, address))
    }

    /// Answers one waiting request, if there is one. A button's POST is
    /// returned for the caller to run, its answer held until `finish`;
    /// STOP with nothing moving is answered at once. Anything else gets the
    /// whole page: the controls, the run's summary, every drive pin's
    /// level, the settings this build runs on, and the serial-port log. A
    /// client that fails mid-answer is closed and dropped; the next poll
    /// goes on.
    pub fn serve(&mut self, delay: &mut Delay, report: &Report, pins: PinStates, log: (&[u8], bool)) -> Option<Action> {
        let socket = self.waiting(delay, false)?;
        let action = self.read_request(delay, socket, false);
        // The experiment runs for hours, longer than a browser waits, so it
        // is answered at once like STOP; the others are held.
        if let Some(action) = action.filter(|&action| action != Action::Stop && action != Action::Experiment) {
            self.held = Some(socket);
            return Some(action);
        }
        if action.is_some() {
            self.send(delay, socket, SEE_OTHER, false);
        } else {
            // The log is the board's own text, so it is always UTF-8.
            let (log, log_full) = log;
            let log = core::str::from_utf8(log).unwrap_or("(log unreadable)");
            let mut page = Buffer::<8192>::new();
            if page.write_str(HTML_HEADER).and_then(|()| page::full(&mut page, report, pins, log, log_full)).is_ok() {
                self.send(delay, socket, page.as_bytes(), false);
            }
        }
        self.close(delay, socket, false);
        action.filter(|&action| action == Action::Experiment)
    }

    /// Answers the held button, once its action has ended: back to the
    /// page, which the browser then fetches with the result on it. If the
    /// browser gave up waiting (STOP pressed on the same page), the send
    /// fails and the close drops it.
    pub fn finish(&mut self, delay: &mut Delay) {
        if let Some(socket) = self.held.take() {
            self.send(delay, socket, SEE_OTHER, false);
            self.close(delay, socket, false);
        }
    }

    /// Sends one finished trial's record to the bench station: `POST
    /// /trial` to [report] `host`:`port`, the CSV line as its body. Only
    /// between trials, with the cart at rest: never during a trial or a
    /// move (user, 2026-10-28). The client socket is the bridge's own
    /// (`cmds_wifi_netif.h`): `+BEGINCLIENT` → `+BEGINCLIENT: <socket>`,
    /// then `+CLIENTCONNECTIP=<socket>,<ip>,<port>`, `+CLIENTSEND` and
    /// `+CLIENTCLOSE` as the page's answers use them. Returns whether the
    /// Pi answered 200.
    pub fn post_trial(&mut self, delay: &mut Delay, record: &str) -> bool {
        let socket = match self.command(delay, b"AT+BEGINCLIENT\r\n", 1_000) {
            Some((n, true)) => after(&self.resp[..n], b"+BEGINCLIENT: ").and_then(parse_i32),
            _ => None,
        };
        let Some(socket) = socket.filter(|&socket| socket >= 0) else {
            return false;
        };
        let mut cmd = Buffer::<64>::new();
        // The connect waits on the ESP32's TCP timeout when the Pi is off.
        let connected = write!(cmd, "AT+CLIENTCONNECTIP={socket},{REPORT_HOST},{REPORT_PORT}\r\n").is_ok()
            && matches!(self.command(delay, cmd.as_bytes(), 5_000), Some((_, true)));
        let mut answered = false;
        let mut request = Buffer::<320>::new();
        if connected
            && write!(
                request,
                "POST /trial HTTP/1.0\r\nHost: {REPORT_HOST}\r\nContent-Type: text/csv\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{record}\n",
                record.len() + 1
            )
            .is_ok()
        {
            self.send(delay, socket, request.as_bytes(), false);
            // The answer's first line: `HTTP/1.1 200 OK` when the Pi took it.
            self.head_len = 0;
            for _ in 0..RECEIVE_ROUNDS * 5 {
                if self.head_len > 0 || self.receive(delay, socket, false).is_none() {
                    break;
                }
                delay.delay_ms(20);
            }
            let head = &self.head[..self.head_len];
            answered = head.starts_with(b"HTTP/1.") && head.get(9..12) == Some(b"200".as_slice());
        }
        self.close(delay, socket, false);
        answered
    }

    /// Called every `stop_check_ms` during a slow move: answers one
    /// waiting request, if there is one, and returns whether it was the
    /// STOP button's POST. Any other request (a reload by hand) gets the
    /// short moving page, whose STOP button works; a button other than
    /// STOP is ignored while the cart moves. The line is silent during a
    /// move, so nothing is drained first, which keeps the pause short:
    /// about 6 ms with no request. Answering one pauses the move for the
    /// time to read it and send the page, which shows as a hitch.
    pub fn stop_requested(&mut self, delay: &mut Delay, moving: Action, progress: Option<&ExperimentReport>) -> bool {
        let held = self.held;
        let Some(socket) = self.waiting(delay, true).filter(|&socket| Some(socket) != held) else {
            return false;
        };
        let stop = self.read_request(delay, socket, true) == Some(Action::Stop);
        if stop {
            self.send(delay, socket, SEE_OTHER, true);
        } else {
            let mut page = Buffer::<3072>::new();
            if page.write_str(HTML_HEADER).and_then(|()| page::moving(&mut page, moving, progress)).is_ok() {
                self.send(delay, socket, page.as_bytes(), true);
            }
        }
        self.close(delay, socket, true);
        stop
    }

    /// The socket of a connected client that has sent something, if any.
    fn waiting(&mut self, delay: &mut Delay, quick: bool) -> Option<i32> {
        let mut cmd = Buffer::<48>::new();
        write!(cmd, "AT+SERVERAVAILABLE={}\r\n", self.server).ok()?;
        match self.ask(delay, cmd.as_bytes(), 1_000, quick) {
            Some((n, true)) => after(&self.resp[..n], b"+SERVERAVAILABLE: ").and_then(parse_i32),
            _ => None,
        }
        .filter(|&socket| socket >= 0)
    }

    /// Reads the whole request, so closing the socket does not reset it,
    /// and returns the action its first line asks for.
    fn read_request(&mut self, delay: &mut Delay, socket: i32, quick: bool) -> Option<Action> {
        self.head_len = 0;
        for _ in 0..RECEIVE_ROUNDS {
            match self.receive(delay, socket, quick) {
                Some(0) | None => break,
                Some(_) => {}
            }
        }
        Action::from_request(&self.head[..self.head_len])
    }

    /// Sends `bytes` to the client. A failure is left to the close.
    fn send(&mut self, delay: &mut Delay, socket: i32, bytes: &[u8], quick: bool) {
        let mut cmd = Buffer::<48>::new();
        if write!(cmd, "AT+CLIENTSEND={socket},{}\r\n", bytes.len()).is_ok() {
            if !quick {
                self.drain(delay);
            }
            self.uart.write_bytes(cmd.as_bytes());
            self.uart.write_bytes(bytes);
            self.reply(delay, 2_000);
        }
    }

    fn close(&mut self, delay: &mut Delay, socket: i32, quick: bool) {
        let mut cmd = Buffer::<48>::new();
        if write!(cmd, "AT+CLIENTCLOSE={socket}\r\n").is_ok() {
            self.ask(delay, cmd.as_bytes(), 1_000, quick);
        }
    }

    /// One `+CLIENTRECEIVE`: reads up to `RECEIVE_CHUNK` bytes of the
    /// request, keeping the first `HEAD_MAX` of the whole request in
    /// `head`. Returns how many arrived. The data is counted, not
    /// scanned, so a request containing "OK" cannot end the reply early.
    fn receive(&mut self, delay: &mut Delay, socket: i32, quick: bool) -> Option<u32> {
        let mut cmd = Buffer::<48>::new();
        write!(cmd, "AT+CLIENTRECEIVE={socket},{RECEIVE_CHUNK}\r\n").ok()?;
        if !quick {
            self.drain(delay);
        }
        self.uart.write_bytes(cmd.as_bytes());
        // The prompt and the count, up to the `|` before the data.
        let mut n = 0;
        let mut waited_us = 0;
        loop {
            match self.uart.read_byte() {
                Ok(byte) => {
                    if n < RESP_MAX {
                        self.resp[n] = byte;
                        n += 1;
                    }
                    if byte == b'|' {
                        break;
                    }
                    if ends_with(&self.resp[..n], b"ERROR\r\n") {
                        return None;
                    }
                }
                Err(ReadError::Empty) => {
                    if waited_us >= 1_000_000 {
                        return None;
                    }
                    delay.delay_us(POLL_US);
                    waited_us += POLL_US;
                }
                Err(_) => {}
            }
        }
        let count = after_until(&self.resp[..n], b"+CLIENTRECEIVE: ", b'|').and_then(parse_u32)?;
        let mut read = 0;
        let mut waited_us = 0;
        while read < count {
            match self.uart.read_byte() {
                Ok(byte) => {
                    if self.head_len < HEAD_MAX {
                        self.head[self.head_len] = byte;
                        self.head_len += 1;
                    }
                    read += 1;
                }
                Err(ReadError::Empty) => {
                    if waited_us >= 1_000_000 {
                        return None;
                    }
                    delay.delay_us(POLL_US);
                    waited_us += POLL_US;
                }
                Err(_) => {}
            }
        }
        matches!(self.reply(delay, 1_000), Some((_, true))).then_some(count)
    }

    /// Sends one AT command and collects its reply; see `reply`.
    fn command(&mut self, delay: &mut Delay, cmd: &[u8], timeout_ms: u32) -> Option<(usize, bool)> {
        self.ask(delay, cmd, timeout_ms, false)
    }

    /// `command`, but with `quick` it skips the 20 ms drain: only during
    /// a move, when the line is silent.
    fn ask(&mut self, delay: &mut Delay, cmd: &[u8], timeout_ms: u32, quick: bool) -> Option<(usize, bool)> {
        if !quick {
            self.drain(delay);
        }
        self.uart.write_bytes(cmd);
        self.reply(delay, timeout_ms)
    }

    /// Collects a reply until `OK\r\n` or `ERROR\r\n`: `(length, ok)`, or
    /// None if the line stays idle for `timeout_ms`.
    fn reply(&mut self, delay: &mut Delay, timeout_ms: u32) -> Option<(usize, bool)> {
        let timeout_us = timeout_ms.saturating_mul(1_000);
        let mut n = 0;
        let mut waited_us = 0;
        while waited_us < timeout_us {
            match self.uart.read_byte() {
                Ok(byte) => {
                    if n < RESP_MAX {
                        self.resp[n] = byte;
                        n += 1;
                    }
                    if ends_with(&self.resp[..n], b"OK\r\n") {
                        return Some((n, true));
                    }
                    if ends_with(&self.resp[..n], b"ERROR\r\n") {
                        return Some((n, false));
                    }
                }
                Err(ReadError::Empty) => {
                    delay.delay_us(POLL_US);
                    waited_us += POLL_US;
                }
                // A framing error or overrun drops the byte; keep reading.
                Err(_) => {}
            }
        }
        None
    }

    /// Discards stale bytes until the line has been quiet for 20 ms.
    fn drain(&mut self, delay: &mut Delay) {
        let mut quiet_us = 0;
        while quiet_us < 20_000 {
            match self.uart.read_byte() {
                Err(ReadError::Empty) => {
                    delay.delay_us(POLL_US);
                    quiet_us += POLL_US;
                }
                Ok(_) | Err(_) => quiet_us = 0,
            }
        }
    }
}

fn ends_with(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.len() >= needle.len() && &bytes[bytes.len() - needle.len()..] == needle
}

/// The value after `prompt`, up to the line end.
fn after<'a>(reply: &'a [u8], prompt: &[u8]) -> Option<&'a [u8]> {
    after_until(reply, prompt, b'\r')
}

/// The value after `prompt`, up to `end` (or the reply's end).
fn after_until<'a>(reply: &'a [u8], prompt: &[u8], end: u8) -> Option<&'a [u8]> {
    let start = reply.windows(prompt.len()).position(|window| window == prompt)? + prompt.len();
    let rest = &reply[start..];
    let stop = rest.iter().position(|&byte| byte == end).unwrap_or(rest.len());
    Some(&rest[..stop])
}

fn parse_u32(text: &[u8]) -> Option<u32> {
    core::str::from_utf8(text).ok()?.trim().parse().ok()
}

fn parse_i32(text: &[u8]) -> Option<i32> {
    core::str::from_utf8(text).ok()?.trim().parse().ok()
}
