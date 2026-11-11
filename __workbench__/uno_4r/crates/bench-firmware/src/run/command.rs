use crate::{
    constants::{COMMAND_NEGATIVE, COMMAND_POSITIVE, COMMAND_SIZE},
    wifi::datagram::Datagram,
};

/// The rate in a 3-byte command, pulses/s: byte 0 the sign (0 negative, 1
/// positive), bytes 1–2 the size, a little-endian u16 (__documentation__/real-loop-dia.md,
/// *Contracts*), parsed as the controller's `wifi/message.rs` does. None for
/// any other datagram, or a bad sign byte, which the bench ignores.
pub(crate) fn rate(datagram: &Datagram) -> Option<i32> {
    if datagram.size() != COMMAND_SIZE {
        return None;
    }
    let [sign, low, high] = *datagram.payload() else {
        return None;
    };
    let size = i32::from(u16::from_le_bytes([low, high]));
    match sign {
        COMMAND_NEGATIVE => Some(-size),
        COMMAND_POSITIVE => Some(size),
        _ => None,
    }
}
