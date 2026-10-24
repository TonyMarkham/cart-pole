use crate::config::Config;

/// One second in the time unit `advance` takes.
const MICROS_PER_SECOND: i64 = 1_000_000;

/// The step generator: holds the commanded pulse rate and turns elapsed
/// time into whole step pulses.
///
/// Rules (each checkable by hand):
/// - The rate is the last command, clamped to ±`Config::max_rate`.
/// - Pulses = rate × elapsed time. The fraction of a pulse left over is
///   carried (in pulse·µs) into the next call, so over any span the
///   pulses emitted equal ∫ rate dt to within one pulse, whatever the
///   call interval.
/// - No pulse takes the position past ±`Config::travel_pulses`; at the
///   envelope the carried fraction is dropped, so the cart stays put
///   until a command points back inside.
#[derive(Clone, Debug)]
pub struct Stepper {
    config: Config,
    /// Pulses from the rail's center; + is the direction a positive rate
    /// moves.
    position: i32,
    /// The commanded rate after the clamp, pulses/s.
    rate: i32,
    /// The carried fraction of a pulse, pulse·µs: |remainder| < 1 s.
    remainder: i64,
}

impl Stepper {
    /// A stepper at rest at `position` pulses from center (clamped to the
    /// envelope).
    pub fn new(config: Config, position: i32) -> Stepper {
        let travel = config.travel_pulses();
        Stepper { config, position: position.clamp(-travel, travel), rate: 0, remainder: 0 }
    }

    /// Takes one command: the Orin's pulse rate in pulses/s (sign =
    /// direction), as decoded by `decode_command`.
    pub fn command(&mut self, rate: i32) {
        self.rate = self.config.clamp_rate(rate);
    }

    /// Advances `micros` µs at the commanded rate and returns the pulses
    /// emitted in that span, signed: what the board's step pin sends, and
    /// what the synthetic build turns into `ctrl`.
    pub fn advance(&mut self, micros: u32) -> i32 {
        let total = self.remainder + i64::from(self.rate) * i64::from(micros);
        // Integer division truncates toward zero, so the remainder keeps
        // the sign of the motion it belongs to.
        let mut pulses = total / MICROS_PER_SECOND;
        self.remainder = total - pulses * MICROS_PER_SECOND;

        let travel = i64::from(self.config.travel_pulses());
        let target = i64::from(self.position) + pulses;
        if target > travel || target < -travel {
            pulses = target.clamp(-travel, travel) - i64::from(self.position);
            self.remainder = 0;
        }
        // |pulses| ≤ 2·travel, which fits an i32 for any rail the Config
        // can describe.
        let pulses = pulses as i32;
        self.position += pulses;
        pulses
    }

    /// Pulses from the rail's center.
    pub fn position(&self) -> i32 {
        self.position
    }

    /// The commanded rate after the clamp, pulses/s.
    pub fn rate(&self) -> i32 {
        self.rate
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
