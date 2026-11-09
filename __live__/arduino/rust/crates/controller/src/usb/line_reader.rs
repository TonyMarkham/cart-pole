use crate::constants::LINE_MAX;

use unor4_hal::uart::UartIrq;

/// Gathers the bytes from the Pi into whole lines. A line longer than
/// `LINE_MAX` is no line the board knows, so it is thrown away whole, up to
/// and including its end.
pub(crate) struct LineReader {
    bytes: [u8; LINE_MAX],
    len: usize,
    /// The line being read ran past `LINE_MAX`, so it is dropped at its end.
    too_long: bool,
}

impl LineReader {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; LINE_MAX],
            len: 0,
            too_long: false,
        }
    }

    /// Takes the bytes waiting in `usb`'s receive ring up to the next `\n`,
    /// and returns the line they end, without the `\n`, or a `\r` before it,
    /// which a serial monitor may send. None once the ring is empty and no
    /// line has ended; the bytes so far are kept for the next call.
    pub(crate) fn read_line(&mut self, usb: &UartIrq<ra4m1::SCI9>) -> Option<&[u8]> {
        while let Some(byte) = usb.read_byte() {
            if byte == b'\n' {
                let len = core::mem::take(&mut self.len);
                if core::mem::take(&mut self.too_long) {
                    continue;
                }
                let line = &self.bytes[..len];
                return Some(line.strip_suffix(b"\r").unwrap_or(line));
            }
            if self.len < LINE_MAX {
                self.bytes[self.len] = byte;
                self.len += 1;
            } else {
                self.too_long = true;
            }
        }
        None
    }
}
