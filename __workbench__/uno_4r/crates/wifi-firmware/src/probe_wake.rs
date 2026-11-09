use crate::usb::command::Command;

/// What woke the board, with the clock's count at the look that found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeWake {
    /// A 3-byte datagram, whole.
    Datagram { payload: [u8; 3], at: u32 },
    /// A line from the PC over USB.
    Usb { command: Command, at: u32 },
}
