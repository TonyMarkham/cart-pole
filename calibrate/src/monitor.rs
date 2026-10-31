use core::fmt::{self, Write};

use unor4_hal::uart::Uart;

use crate::buffer::Buffer;

/// The log's size, bytes: one run prints well under 1 KB, so a full
/// log means something printed far more than a run should.
const LOG_BYTES: usize = 2048;

/// Lines of text on the monitor line (SCI9), which the board's ESP32-S3
/// forwards to its USB serial port, with a copy of every byte kept so
/// the WiFi page can show the whole log without a USB cable. Nothing
/// listens for an answer, so a write never fails in a way the program
/// could act on.
pub struct Monitor {
    uart: Uart<ra4m1::SCI9>,
    log: Buffer<LOG_BYTES>,
    log_full: bool,
}

impl Monitor {
    pub fn new(uart: Uart<ra4m1::SCI9>) -> Self {
        Self {
            uart,
            log: Buffer::new(),
            log_full: false,
        }
    }

    /// Writes one line, ending it with CR LF.
    pub fn line(&mut self, args: fmt::Arguments) {
        // write_str below never returns an error.
        let _ = self.write_fmt(args);
        let _ = self.write_str("\r\n");
    }

    /// Everything printed so far, and whether the log filled up (the
    /// serial port still got every line).
    pub fn log(&self) -> (&[u8], bool) {
        (self.log.as_bytes(), self.log_full)
    }
}

impl Write for Monitor {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.uart.write_bytes(text.as_bytes());
        if !self.log_full && self.log.write_str(text).is_err() {
            self.log_full = true;
        }
        Ok(())
    }
}
