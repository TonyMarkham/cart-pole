use crate::constants::STOP_FRAME_END;

use unor4_hal::uart::UartIrq;

/// Listens for the `Stop` datagram during a move. Each byte from the WiFi chip
/// waits in the UART's receive ring, put there by its interrupt, which also
/// ends the move's sleep, so the move looks at the ring each time it wakes.
/// Every other byte is thrown away, as a move has always done.
pub struct StopListener {
    at: &'static UartIrq<ra4m1::SCI1>,
    /// How much of `STOP_FRAME_END` the latest bytes have matched.
    matched: usize,
}

impl StopListener {
    pub fn new(at: &'static UartIrq<ra4m1::SCI1>) -> Self {
        Self { at, matched: 0 }
    }

    /// Whether the whole of a `Stop` frame has arrived. The header's end `|`
    /// comes only first in `STOP_FRAME_END`, so a failed match can restart only
    /// from there. The line end must match too, so a longer datagram that
    /// merely starts with `Stop` is not one.
    pub fn heard(&mut self) -> bool {
        while let Some(byte) = self.at.read_byte() {
            if byte == STOP_FRAME_END[self.matched] {
                self.matched += 1;
            } else {
                self.matched = usize::from(byte == STOP_FRAME_END[0]);
            }
            if self.matched == STOP_FRAME_END.len() {
                self.matched = 0;
                return true;
            }
        }
        false
    }
}
