use crate::protocol::LINE_END;

use anyhow::{Context, Result};
use serialport::SerialPort;
use std::{
    io::{ErrorKind, Read, Write},
    time::{Duration, Instant},
};

/// How long one read waits for a byte before the deadline is checked again,
/// ms. Only bounds how late a `read_line` notices its own deadline; a byte
/// that arrives ends the wait at once.
const READ_SLICE_MS: u64 = 20;

/// The most bytes one read takes, bytes. Any size works, since the bytes
/// are kept until their line ends: this one holds a whole report line many
/// times over.
const READ_BUFFER_BYTES: usize = 256;

/// The byte a board may send before `LINE_END`, dropped from each line:
/// Arduino's `Serial.println` ends lines with `\r\n`.
const CARRIAGE_RETURN: char = '\r';

/// One board's USB serial line, read a line at a time: `LINE_END` ends a
/// line, and a `\r` before it is dropped.
pub(crate) struct SerialLine {
    port: Box<dyn SerialPort>,
    /// The port's name, for messages.
    name: String,
    /// Bytes read whose line hasn't ended yet.
    pending: Vec<u8>,
}

impl SerialLine {
    /// Opens the port with DTR and RTS on. The R4's USB bridge passes nothing
    /// either way until DTR is on: on the bench on 2026-11-07, `STATUS` got
    /// no answer with them off and was answered at once with them on. The
    /// `serialport` crate leaves them off on Windows.
    pub(crate) fn open(name: &str, baud: u32) -> Result<Self> {
        let mut port = serialport::new(name, baud)
            .timeout(Duration::from_millis(READ_SLICE_MS))
            .open()
            .with_context(|| format!("opening {name} at {baud} baud"))?;
        port.write_data_terminal_ready(true)
            .and_then(|()| port.write_request_to_send(true))
            .with_context(|| format!("setting DTR and RTS on {name}"))?;
        Ok(Self {
            port,
            name: name.to_owned(),
            pending: Vec::new(),
        })
    }

    pub(crate) fn write_line(&mut self, text: &str) -> Result<()> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(LINE_END);
        self.port
            .write_all(&bytes)
            .and_then(|()| self.port.flush())
            .with_context(|| format!("writing to {}", self.name))
    }

    /// The next whole line, or None if none ended within `timeout`. Bytes of a
    /// line not yet ended are kept for the next call.
    pub(crate) fn read_line(&mut self, timeout: Duration) -> Result<Option<String>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(end) = self.pending.iter().position(|&byte| byte == LINE_END) {
                let line: Vec<u8> = self.pending.drain(..=end).collect();
                let text = String::from_utf8_lossy(&line[..end]);
                return Ok(Some(text.trim_end_matches(CARRIAGE_RETURN).to_owned()));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            let mut buffer = [0; READ_BUFFER_BYTES];
            match self.port.read(&mut buffer) {
                Ok(read) => self.pending.extend_from_slice(&buffer[..read]),
                Err(error) if error.kind() == ErrorKind::TimedOut => {}
                Err(error) => {
                    return Err(error).with_context(|| format!("reading {}", self.name));
                }
            }
        }
    }
}
