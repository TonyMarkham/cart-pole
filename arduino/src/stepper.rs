use crate::{config::Config, ramp::Ramp};

/// One second in the time unit `advance` takes.
const MICROS_PER_SECOND: i64 = 1_000_000;

/// The step generator: holds the command and the sent rate (`Ramp`) and
/// turns elapsed time into whole step pulses.
///
/// Rules (each checkable by hand):
/// - The command is the last one, clamped to ±`Config::max_rate`.
/// - Each `advance` is one tick of the ramp: the sent rate first moves
///   towards the command by at most m × the elapsed time (`Ramp::tick`),
///   then runs for the whole span.
/// - Pulses = sent rate × elapsed time. The fraction of a pulse left over is
///   carried (in pulse·µs) into the next call, so over any span the
///   pulses emitted equal ∫ rate dt to within one pulse, whatever the
///   call interval.
/// - No pulse takes the position past ±`travel` (the board's step-counting
///   envelope, given at `new`); at the envelope the carried fraction is
///   dropped, so the cart stays put until a command points back inside.
#[derive(Clone, Debug)]
pub struct Stepper {
    config: Config,
    /// The envelope either side of the rail's center, pulses.
    travel: i32,
    /// Pulses from the rail's center; + is the direction a positive rate
    /// moves.
    position: i32,
    /// The command and the sent rate (the acceleration ceiling).
    ramp: Ramp,
    /// The carried fraction of a pulse, pulse·µs: |remainder| < 1 s.
    remainder: i64,
}

impl Stepper {
    /// A stepper at rest at `position` pulses from center, with an
    /// envelope of ±`travel` pulses, `travel` ≥ 0 (`position` is clamped
    /// to it).
    pub fn new(config: Config, travel: i32, position: i32) -> Stepper {
        Stepper { config, travel, position: position.clamp(-travel, travel), ramp: Ramp::new(), remainder: 0 }
    }

    /// Takes one command: the Orin's pulse rate in pulses/s (sign =
    /// direction), as decoded by `decode_command`. The sent rate moves
    /// towards it from the next `advance`.
    pub fn command(&mut self, rate: i32) {
        self.ramp.command(&self.config, rate);
    }

    /// Advances `micros` µs, one tick of the ramp, and returns the pulses
    /// emitted in that span, signed: what the board's step pin sends.
    pub fn advance(&mut self, micros: u32) -> i32 {
        let rate = self.ramp.tick(&self.config, micros);
        let total = self.remainder + i64::from(rate) * i64::from(micros);
        // Integer division truncates toward zero, so the remainder keeps
        // the sign of the motion it belongs to.
        let mut pulses = total / MICROS_PER_SECOND;
        self.remainder = total - pulses * MICROS_PER_SECOND;

        let travel = i64::from(self.travel);
        let target = i64::from(self.position) + pulses;
        if target > travel || target < -travel {
            pulses = target.clamp(-travel, travel) - i64::from(self.position);
            self.remainder = 0;
        }
        // |pulses| ≤ 2·travel, which fits an i32 while travel < 2^30.
        let pulses = pulses as i32;
        self.position += pulses;
        pulses
    }

    /// Pulses from the rail's center.
    pub fn position(&self) -> i32 {
        self.position
    }

    /// The sent rate, the STEP signal's, pulses/s.
    pub fn rate(&self) -> i32 {
        self.ramp.rate()
    }

    /// The last command after the clamp, pulses/s.
    pub fn command_rate(&self) -> i32 {
        self.ramp.command_rate()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
