use crate::{
    command::{COMMAND_LEN, decode_command},
    config::Config,
    limit_switches::LimitSwitches,
    ramp::Ramp,
    signals::Dm542Signals,
    tick_answer::TickAnswer,
};

/// The firmware's sim-loop edge (synthetic-arduino-plan.md, the synthetic
/// build): the same command decode, clamp and ramp as the board, with the
/// radio replaced by `receive` and the pins by `tick`'s answer. The loop's
/// clock is MuJoCo's timestep: the orchestrator ticks it once every
/// timestep, `tick_micros` long (A7). It keeps no pulse count (A5); the
/// rail's ends are its limit switches (A8).
///
/// Rules (each checkable by hand):
/// - `receive` sets the command and nothing else; the sent rate moves only
///   at the next `tick`. In FAULT a command is not applied.
/// - A `tick` given a closed switch enters FAULT: sent rate and command 0,
///   the cart stopped in one jump. Otherwise it ramps (`Ramp::tick`).
/// - In FAULT every tick answers FAULT with STEP rate 0, whatever the
///   switches; only `reset` leaves it.
#[derive(Clone, Debug)]
pub struct SyntheticArduino {
    config: Config,
    /// One tick, µs: MuJoCo's timestep.
    tick_micros: u32,
    ramp: Ramp,
    fault: bool,
}

impl SyntheticArduino {
    /// Created with its `Config` and the loop's tick (synthetic-loop-dia.md,
    /// 3): every run starts the cart at 50% of the rail, a guarantee that
    /// stands in for the board's homing and calibration, so no start
    /// position is taken. At rest: command 0, sent rate 0, not in FAULT.
    pub fn new(config: Config, tick_micros: u32) -> SyntheticArduino {
        SyntheticArduino {
            config,
            tick_micros,
            ramp: Ramp::new(),
            fault: false,
        }
    }

    /// One command packet (19: 4 bytes, `i32` LE, pulses/s), on the
    /// timestep it lands: clamped to the top rate and kept as the command
    /// the ramp heads for. Ignored in FAULT.
    pub fn receive(&mut self, packet: [u8; COMMAND_LEN]) {
        if !self.fault {
            self.ramp.command(&self.config, decode_command(packet));
        }
    }

    /// One timestep's tick (19–20): the limit switches first, then the
    /// ramp. Returns the DM542 signals of the sent rate after it and
    /// whether it is in FAULT.
    pub fn tick(&mut self, switches: LimitSwitches) -> TickAnswer {
        if !self.fault && switches.tripped() {
            self.fault = true;
            self.ramp = Ramp::new();
        }
        let rate = match self.fault {
            true => 0,
            false => self.ramp.tick(&self.config, self.tick_micros),
        };
        TickAnswer {
            signals: Dm542Signals::from_rate(rate),
            fault: self.fault,
        }
    }

    /// R3: back to rest, command 0 and sent rate 0, as MuJoCo's ctrl 0, and
    /// out of FAULT.
    pub fn reset(&mut self) {
        self.ramp = Ramp::new();
        self.fault = false;
    }

    pub fn fault(&self) -> bool {
        self.fault
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
