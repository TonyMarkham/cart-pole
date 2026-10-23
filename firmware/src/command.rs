/// The Orin → Arduino command packet's length (loop-plan.md, *Command*).
pub const COMMAND_LEN: usize = 4;

/// Decodes one command packet: one `i32`, little-endian, the target step
/// pulse rate in pulses per second; sign = direction. Fixed length, no
/// id, so every 4 bytes are one command.
pub fn decode_command(packet: [u8; COMMAND_LEN]) -> i32 {
    i32::from_le_bytes(packet)
}

/// The inverse, for the sender's side (the Orin, or the loop's tests).
pub fn encode_command(rate: i32) -> [u8; COMMAND_LEN] {
    rate.to_le_bytes()
}
