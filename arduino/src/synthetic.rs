use crate::{
    command::{COMMAND_LEN, decode_command},
    config::Config,
    ramp::Ramp,
    signals::Dm542Signals,
};

/// The firmware's sim-loop edge (synthetic-arduino-plan.md, the synthetic
/// build): the same command decode, clamp and ramp as the board, with the
/// radio replaced by `receive`, which returns the DM542 signals instead
/// of driving the pins. The loop's clock is MuJoCo's step: the
/// orchestrator ticks the ramp once a step, `tick_micros` long (A7). It
/// keeps no pulse count (A5): the orchestrator turns the signals into
/// MuJoCo's cart speed, and the travel limit acts only on the board.
#[derive(Clone, Debug)]
pub struct SyntheticArduino {
    config: Config,
    /// One tick, µs: MuJoCo's timestep.
    tick_micros: u32,
    ramp: Ramp,
}

impl SyntheticArduino {
    /// Created with its `Config` and the loop's tick (synthetic-loop-dia.md,
    /// 3): every run starts the cart at 50% of the rail, a guarantee that
    /// stands in for the board's homing, so no start position is taken.
    /// At rest: command 0, sent rate 0.
    pub fn new(config: Config, tick_micros: u32) -> SyntheticArduino {
        SyntheticArduino { config, tick_micros, ramp: Ramp::new() }
    }

    /// One command packet (19: 4 bytes, `i32` LE, pulses/s), clamped to
    /// the top rate, then the arrival step's tick; returns the DM542
    /// signals of the sent rate after it (20).
    pub fn receive(&mut self, packet: [u8; COMMAND_LEN]) -> Dm542Signals {
        self.ramp.command(&self.config, decode_command(packet));
        self.tick()
    }

    /// One tick of every other step (the note at 15–16): the sent rate
    /// moves towards the last command; returns its DM542 signals.
    pub fn tick(&mut self) -> Dm542Signals {
        Dm542Signals::from_rate(self.ramp.tick(&self.config, self.tick_micros))
    }

    /// R3: back to rest, command 0 and sent rate 0, as MuJoCo's ctrl 0.
    pub fn reset(&mut self) {
        self.ramp = Ramp::new();
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
