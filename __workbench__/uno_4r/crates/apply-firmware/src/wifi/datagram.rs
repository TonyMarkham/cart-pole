use crate::constants::DATAGRAM_MAX;

/// One datagram the bridge pushed: its size, as its header gave it, and the
/// start of its payload. The size is what tells a command from a text
/// message: a command is exactly 3 bytes (__documentation__/real-loop-dia.md, *Contracts*).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Datagram {
    bytes: [u8; DATAGRAM_MAX],
    kept: usize,
    size: u32,
}

impl Datagram {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; DATAGRAM_MAX],
            kept: 0,
            size: 0,
        }
    }

    /// Starts a datagram of `size` bytes, forgetting the one before.
    pub(crate) fn start(&mut self, size: u32) {
        self.size = size;
        self.kept = 0;
    }

    /// Takes the payload's next byte, keeping it while there is room.
    pub(crate) fn push(&mut self, byte: u8) {
        if self.kept < DATAGRAM_MAX {
            self.bytes[self.kept] = byte;
            self.kept += 1;
        }
    }

    /// The payload's size in bytes, as the header gave it.
    pub(crate) fn size(&self) -> u32 {
        self.size
    }

    /// The payload's first `DATAGRAM_MAX` bytes: all of it, for any datagram
    /// the board knows.
    pub(crate) fn payload(&self) -> &[u8] {
        &self.bytes[..self.kept]
    }
}
