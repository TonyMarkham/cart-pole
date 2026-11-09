use crate::constants::ANSWER_MAX;

use core::fmt;

/// Room to write one answer before it is sent: `AT+UDPWRITE` needs its size
/// first. Writing more than `ANSWER_MAX` bytes fails, and writes nothing past
/// it.
pub(crate) struct AnswerBuffer {
    bytes: [u8; ANSWER_MAX],
    len: usize,
}

impl AnswerBuffer {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; ANSWER_MAX],
            len: 0,
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl fmt::Write for AnswerBuffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.len + text.len();
        if end > ANSWER_MAX {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
