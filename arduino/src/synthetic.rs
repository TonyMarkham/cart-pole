use crate::{
    command::{COMMAND_LEN, decode_command},
    config::Config,
    stepper::Stepper,
};

/// The firmware's sim-loop edge (synthetic-arduino-plan.md, the synthetic
/// build): the same command decode and step generator as the board, with
/// the radio replaced by `receive` and the step pin and timer replaced by
/// `tick`, whose return value is the step signal the orchestrator turns
/// into MuJoCo's actuator value.
#[derive(Clone, Debug)]
pub struct SyntheticArduino {
    stepper: Stepper,
}

/// The step signal over one tick: `pulses` step pulses (signed: + is the
/// positive rate's direction) emitted in `micros` µs. The pulse train's
/// mean rate over the tick is pulses / micros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepSignal {
    pub pulses: i32,
    pub micros: u32,
}

impl SyntheticArduino {
    /// At rest at the rail's center, 0 pulses (synthetic-loop-dia.md, 3):
    /// every run starts the cart at 50% of the rail, a guarantee that
    /// stands in for the board's homing, so no start position is taken.
    pub fn new(config: Config) -> SyntheticArduino {
        SyntheticArduino { stepper: Stepper::new(config, 0) }
    }

    /// One command packet from the Orin (4 bytes, `i32` LE, pulses/s).
    pub fn receive(&mut self, packet: [u8; COMMAND_LEN]) {
        self.stepper.command(decode_command(packet));
    }

    /// Runs `micros` µs and returns the step signal emitted in that span.
    pub fn tick(&mut self, micros: u32) -> StepSignal {
        StepSignal { pulses: self.stepper.advance(micros), micros }
    }

    pub fn stepper(&self) -> &Stepper {
        &self.stepper
    }
}
