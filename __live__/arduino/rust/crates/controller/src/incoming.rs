use crate::{usb::command::Command as UsbCommand, wifi::datagram::Datagram};

/// What woke the board with something to do: a line from the Pi over USB, a
/// datagram from the Orin over WiFi, or, in CALIBRATED, a limit switch.
pub(crate) enum Incoming {
    Usb(UsbCommand),
    Wifi(Datagram),
    /// A limit switch was pressed, or reads pressed, in CALIBRATED: a fault.
    Switch,
}
