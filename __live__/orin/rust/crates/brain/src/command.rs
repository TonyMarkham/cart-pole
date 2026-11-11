use crate::constants::{COMMAND_NEGATIVE, COMMAND_POSITIVE, COMMAND_SIZE};

/// The command for the cart: a pulse rate in pulses/s, its sign the direction,
/// c = round(a × the top pulse rate) (`__documentation__/real-loop-dia.md`, *Contracts*, the
/// policy's output). On the wire it is 3 bytes: the sign, 0 negative and 1
/// positive, then the rate's size as a `u16`, little-endian (*Contracts*, the
/// command). A rate of 0 means rest, and is sent with the positive sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Command {
    pub(crate) rate: i32,
}

impl Command {
    /// The policy's a, clipped to [−1, 1] as `policy.py` clips it, times the
    /// top rate, rounded to the nearest whole pulse per second.
    pub(crate) fn from_action(a: f64, top_pulse_rate: u32) -> Command {
        let rate = (a.clamp(-1.0, 1.0) * f64::from(top_pulse_rate)).round();
        Command { rate: rate as i32 }
    }

    /// The datagram. The size is the top rate at most, which fits a `u16`
    /// (`PolicyValues::check`), so nothing is lost in the cast.
    pub(crate) fn to_bytes(self) -> [u8; COMMAND_SIZE] {
        let sign = if self.rate < 0 {
            COMMAND_NEGATIVE
        } else {
            COMMAND_POSITIVE
        };
        let size = u16::try_from(self.rate.unsigned_abs()).unwrap_or(u16::MAX);
        let [low, high] = size.to_le_bytes();
        [sign, low, high]
    }
}
