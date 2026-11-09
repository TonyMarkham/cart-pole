use crate::constants::{REPLY_ERROR, REPLY_MAX, REPLY_OK};

/// One reply from the WiFi chip to an AT command: its lines up to the one that
/// ends it, `OK` or `ERROR`. Bytes past `REPLY_MAX` aren't kept, but the reply
/// still ends at its last line.
pub(crate) struct Reply {
    bytes: [u8; REPLY_MAX],
    len: usize,
    /// The start of the line arriving now: as much as the longest last line,
    /// `ERROR\r\n`, so the last line can be told even when `bytes` is full.
    line: [u8; REPLY_ERROR.len()],
    line_len: usize,
    /// The line arriving now is longer than `line`, so it isn't a last line.
    line_long: bool,
    ok: bool,
}

impl Reply {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; REPLY_MAX],
            len: 0,
            line: [0; REPLY_ERROR.len()],
            line_len: 0,
            line_long: false,
            ok: false,
        }
    }

    /// Takes the next byte from the chip, and returns true once it ended the
    /// reply. A line ends the reply only if it is all `OK\r\n` or all
    /// `ERROR\r\n`.
    pub(crate) fn push(&mut self, byte: u8) -> bool {
        if self.len < REPLY_MAX {
            self.bytes[self.len] = byte;
            self.len += 1;
        }
        if self.line_len < self.line.len() {
            self.line[self.line_len] = byte;
            self.line_len += 1;
        } else {
            self.line_long = true;
        }
        if byte != b'\n' {
            return false;
        }
        let len = core::mem::take(&mut self.line_len);
        if core::mem::take(&mut self.line_long) {
            return false;
        }
        let line = &self.line[..len];
        if line == REPLY_OK {
            self.ok = true;
            return true;
        }
        line == REPLY_ERROR
    }

    /// Whether the reply ended in `OK`: the chip did the command.
    pub(crate) fn ok(&self) -> bool {
        self.ok
    }

    /// The value after `prompt`, up to its line's end: in `+GETSTATUS: 3`,
    /// the prompt `+GETSTATUS: ` gives `3`.
    pub(crate) fn after(&self, prompt: &[u8]) -> Option<&[u8]> {
        let bytes = &self.bytes[..self.len];
        let start = bytes
            .windows(prompt.len())
            .position(|window| window == prompt)?
            + prompt.len();
        let rest = &bytes[start..];
        let stop = rest
            .iter()
            .position(|&byte| byte == b'\r')
            .unwrap_or(rest.len());
        Some(&rest[..stop])
    }

    /// The number after `prompt`, as `after` finds it.
    pub(crate) fn number_after<T: core::str::FromStr>(&self, prompt: &[u8]) -> Option<T> {
        core::str::from_utf8(self.after(prompt)?)
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    /// Forgets the line arriving now: a datagram's frame broke into it, and
    /// its first bytes, `+UDPRX: …`, aren't part of the reply.
    pub(crate) fn drop_line(&mut self) {
        self.line_len = 0;
        self.line_long = false;
    }

    /// The text after `prompt`, as `after` finds it, if it is UTF-8.
    pub(crate) fn text_after(&self, prompt: &[u8]) -> Option<&str> {
        core::str::from_utf8(self.after(prompt)?).ok()
    }
}
