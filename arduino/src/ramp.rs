use crate::config::Config;

/// The acceleration ceiling (synthetic-loop-dia.md, *The acceleration
/// ceiling*): the two rates the firmware holds, and the one rule that
/// joins them, the same on the board and in the loop.
///
/// Rules (each checkable by hand):
/// - The command is the last one received, clamped to ±`Config::max_rate`.
/// - Each tick of `micros` µs moves the sent rate towards the command by
///   at most `Config::ramp_limit(micros)`, m × the tick rounded down, so
///   the sent rate never changes faster than m.
/// - A change within that limit finishes in the tick that takes it; the
///   sent rate never passes the command.
/// - Both start at 0, the stepper at rest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ramp {
    /// The last command after the clamp, pulses/s.
    command: i32,
    /// The rate the STEP signal runs at, pulses/s.
    rate: i32,
}

impl Ramp {
    /// At rest: command 0, sent rate 0.
    pub const fn new() -> Ramp {
        Ramp {
            command: 0,
            rate: 0,
        }
    }

    /// Takes one command, pulses/s (sign = direction), clamped to the top
    /// rate. The sent rate moves only at the next `tick`.
    pub fn command(&mut self, config: &Config, rate: i32) {
        self.command = config.clamp_rate(rate);
    }

    /// One tick of `micros` µs: the sent rate moves towards the command by
    /// at most m × the tick. Returns the sent rate for that tick.
    pub fn tick(&mut self, config: &Config, micros: u32) -> i32 {
        // ramp_limit ≤ u32::MAX × u32::MAX / 10^6, inside an i64.
        let most = config.ramp_limit(micros) as i64;
        let change = (i64::from(self.command) - i64::from(self.rate)).clamp(-most, most);
        // Between the old sent rate and the command, both i32.
        self.rate = (i64::from(self.rate) + change) as i32;
        self.rate
    }

    /// The last command after the clamp, pulses/s.
    pub fn command_rate(&self) -> i32 {
        self.command
    }

    /// The sent rate, pulses/s.
    pub fn rate(&self) -> i32 {
        self.rate
    }
}
