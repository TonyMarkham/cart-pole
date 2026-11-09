use crate::constants::{
    BITS_PER_HEX_DIGIT, BYTE_HEX_DIGITS, HEX_DIGIT_MASK, HEX_RADIX, LINE_MAX, U32_HEX_DIGITS,
};

/// The hex digits, by value.
const HEX: &[u8; HEX_RADIX as usize] = b"0123456789ABCDEF";

/// One line being built for the USB line, with numbers as fixed-width hex:
/// fixed width keeps every `SYNC` answer the same length as its request, and
/// writing hex is a few shifts, so the stamp written last costs almost
/// nothing between being taken and being sent.
pub(crate) struct HexLine {
    bytes: [u8; LINE_MAX],
    len: usize,
}

impl HexLine {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; LINE_MAX],
            len: 0,
        }
    }

    /// Adds `text`. Bytes beyond `LINE_MAX` are dropped; no line here comes
    /// near it.
    pub(crate) fn text(&mut self, text: &[u8]) -> &mut Self {
        for &byte in text {
            self.byte(byte);
        }
        self
    }

    /// Adds `value` as 8 hex digits, the most significant first.
    pub(crate) fn u32(&mut self, value: u32) -> &mut Self {
        for place in (0..U32_HEX_DIGITS as u32).rev() {
            self.digit(value >> (BITS_PER_HEX_DIGIT * place));
        }
        self
    }

    /// Adds each of `bytes` as 2 hex digits.
    pub(crate) fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for &value in bytes {
            for place in (0..BYTE_HEX_DIGITS as u32).rev() {
                self.digit(u32::from(value) >> (BITS_PER_HEX_DIGIT * place));
            }
        }
        self
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Adds the hex digit of `value`'s lowest 4 bits.
    fn digit(&mut self, value: u32) {
        self.byte(HEX[(value & HEX_DIGIT_MASK) as usize]);
    }

    fn byte(&mut self, byte: u8) {
        if self.len < LINE_MAX {
            self.bytes[self.len] = byte;
            self.len += 1;
        }
    }
}
