use core::fmt::{self, Write};
use unor4_hal::uart::Uart;

/// Lines of text on the monitor line (SCI9), which the board's ESP32-S3
/// forwards to its USB serial port. `Uart::write_bytes` cannot fail.
pub struct Monitor(pub Uart<ra4m1::SCI9>);

impl Write for Monitor {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write_bytes(text.as_bytes());
        Ok(())
    }
}
