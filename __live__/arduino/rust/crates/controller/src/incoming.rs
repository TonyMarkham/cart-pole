use crate::{usb::command::Command as UsbCommand, wifi::datagram::Datagram};

/// What woke the board with something to do: a line from the Pi over USB, or
/// a datagram from the Orin over WiFi.
pub(crate) enum Incoming {
    Usb(UsbCommand),
    Wifi(Datagram),
}
