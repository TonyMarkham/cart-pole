pub(crate) mod command;
pub(crate) mod line_reader;

// ---------------------------------------------------------------------------------------------- //

use crate::usb::{command::Command, line_reader::LineReader};

use unor4_hal::uart::UartIrq;

/// The USB serial line to the PC: SCI9, through the board's ESP32-S3 bridge
/// to the USB connector, as in the controller. A `static`, because the HAL's
/// interrupt handlers reach it after `attach` returns. Each byte from the PC
/// lands in its receive ring and wakes the processor from `WFI`.
pub(crate) static USB: UartIrq<ra4m1::SCI9> = UartIrq::new();

/// Takes the lines waiting on the USB line, without sleeping, and returns the
/// first one the board knows. Lines it doesn't know are dropped unanswered.
pub(crate) fn look(reader: &mut LineReader) -> Option<Command> {
    while let Some(line) = reader.read_line(&USB) {
        if let Some(command) = Command::parse(line) {
            return Some(command);
        }
    }
    None
}
