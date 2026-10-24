use crate::{
    command::{COMMAND_LEN, decode_command},
    config::Config,
    signals::Drv8825Signals,
};

/// The firmware's sim-loop edge (synthetic-arduino-plan.md, the synthetic
/// build): the same command decode and clamp as the board, with the radio
/// replaced by `receive`, which returns the DRV8825 signals instead of
/// driving the pins. There is no clock in the loop, so it keeps no pulse
/// count (A5): the orchestrator turns the signals into MuJoCo's cart
/// speed, and the travel limit acts only on the board.
#[derive(Clone, Debug)]
pub struct SyntheticArduino {
    config: Config,
}

impl SyntheticArduino {
    /// Created with the rig's numbers alone (synthetic-loop-dia.md, 3):
    /// every run starts the cart at 50% of the rail, a guarantee that
    /// stands in for the board's homing, so no start position is taken.
    pub fn new(config: Config) -> SyntheticArduino {
        SyntheticArduino { config }
    }

    /// One command packet (19: 4 bytes, `i32` LE, pulses/s), clamped to
    /// the top rate; returns the DRV8825 signals it sets (20).
    pub fn receive(&self, packet: [u8; COMMAND_LEN]) -> Drv8825Signals {
        Drv8825Signals::from_rate(self.config.clamp_rate(decode_command(packet)))
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
