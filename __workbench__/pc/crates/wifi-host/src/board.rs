use crate::{
    protocol::{
        FIELD_SEPARATOR, HEX_RADIX, PAYLOAD_HEX_DIGITS, PING_PREFIX, SYNC_PLACEHOLDER, SYNC_PREFIX,
    },
    serial_line::SerialLine,
    sync_exchange::SyncExchange,
};

use anyhow::Result;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// The R4's USB line during the test, and the PC's clock: ns since `start`
/// on `Instant`, the system's monotonic clock. Every PC stamp is read here,
/// so all of them are on that one clock. A report that arrives while the PC
/// waits for something else is kept, not lost.
pub(crate) struct Board {
    line: SerialLine,
    start: Instant,
    /// Reports read so far: payload → the R4's stamp.
    reports: HashMap<[u8; 3], u32>,
    /// Lines that were neither an awaited answer nor a report, for the record.
    pub(crate) strays: Vec<String>,
}

impl Board {
    pub(crate) fn new(line: SerialLine) -> Self {
        Self {
            line,
            start: Instant::now(),
            reports: HashMap::new(),
            strays: Vec::new(),
        }
    }

    /// The PC's clock, ns since the run started.
    pub(crate) fn now_ns(&self) -> f64 {
        self.start.elapsed().as_nanos() as f64
    }

    /// One sync numbered `number`, or None if no answer came in `timeout`.
    pub(crate) fn sync(&mut self, number: u32, timeout: Duration) -> Result<Option<SyncExchange>> {
        let request = format!("{SYNC_PREFIX}{number:08X}{FIELD_SEPARATOR}{SYNC_PLACEHOLDER}");
        let awaited = format!("{SYNC_PREFIX}{number:08X}{FIELD_SEPARATOR}");
        let t0_ns = self.now_ns();
        self.line.write_line(&request)?;
        let deadline = Instant::now() + timeout;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            let Some(line) = self.line.read_line(left)? else {
                break;
            };
            let t3_ns = self.now_ns();
            if let Some(stamps) = line.strip_prefix(&awaited)
                && let Some((t1, t2)) = stamps.split_once(FIELD_SEPARATOR)
                && let (Ok(t1), Ok(t2)) = (
                    u32::from_str_radix(t1, HEX_RADIX),
                    u32::from_str_radix(t2, HEX_RADIX),
                )
            {
                return Ok(Some(SyncExchange {
                    number,
                    t0_ns,
                    t1,
                    t2,
                    t3_ns,
                }));
            }
            self.take(line);
        }
        Ok(None)
    }

    /// Waits up to `timeout` for the report of `payload`, and returns its
    /// stamp, or None.
    pub(crate) fn report(&mut self, payload: [u8; 3], timeout: Duration) -> Result<Option<u32>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(&stamp) = self.reports.get(&payload) {
                return Ok(Some(stamp));
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return Ok(None);
            };
            let Some(line) = self.line.read_line(left)? else {
                return Ok(None);
            };
            self.take(line);
        }
    }

    /// A late report's stamp, if one came after its wait ended.
    pub(crate) fn late_report(&self, payload: [u8; 3]) -> Option<u32> {
        self.reports.get(&payload).copied()
    }

    /// Reads whatever is left for `timeout`, so late reports are kept.
    pub(crate) fn drain(&mut self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.line.read_line(left)? {
                Some(line) => self.take(line),
                None => break,
            }
        }
        Ok(())
    }

    /// Keeps a report, or records the line as a stray.
    fn take(&mut self, line: String) {
        if let Some((payload, stamp)) = parse_report(&line) {
            self.reports.insert(payload, stamp);
        } else {
            self.strays.push(line);
        }
    }
}

/// `PING <6 hex digits> <8 hex digits>` as (payload, stamp).
pub(crate) fn parse_report(line: &str) -> Option<([u8; 3], u32)> {
    let (payload, stamp) = line
        .strip_prefix(PING_PREFIX)?
        .split_once(FIELD_SEPARATOR)?;
    if payload.len() != PAYLOAD_HEX_DIGITS {
        return None;
    }
    let packed = u32::from_str_radix(payload, HEX_RADIX).ok()?;
    let [_, a, b, c] = packed.to_be_bytes();
    Some(([a, b, c], u32::from_str_radix(stamp, HEX_RADIX).ok()?))
}
