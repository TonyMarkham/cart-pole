pub(crate) mod command;
pub(crate) mod line_reader;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    sleep,
    usb::{command::Command, line_reader::LineReader},
};

use unor4_hal::uart::UartIrq;

/// The USB serial line to the Pi's `arduino-manager`: SCI9, through the
/// board's ESP32-S3 bridge to the USB connector (real-open-issues.md, issue
/// 10, step 4). A `static`, because the HAL's interrupt handlers reach it
/// after `attach` returns (unor4-hal `uart/uart_irq.rs`). Each byte from the
/// Pi lands in its receive ring and wakes the processor from `WFI`. The
/// rings are the HAL's defaults: 256 bytes in, 128 out.
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

/// Sleeps until a line the board knows has arrived on the USB line, and
/// returns its command.
pub(crate) fn next_command(reader: &mut LineReader) -> Command {
    sleep::until(|| look(reader))
}
