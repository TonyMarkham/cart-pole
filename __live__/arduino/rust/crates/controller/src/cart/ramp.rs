use crate::{
    config::motion::{JUMP_LIMIT, TOP_RATE},
    constants::MICROS_PER_SECOND,
};

use unor4_hal::step::TICK_US;

/// The most one tick moves the sent rate, pulses/s: m × the 2 ms tick,
/// rounded down. 1,210,937 × 0.002 = 2,421.
const STEP: i64 = JUMP_LIMIT as i64 * TICK_US as i64 / MICROS_PER_SECOND as i64;

/// The ramp: the two rates the board holds, and the one rule that joins them,
/// copied from the synthetic Arduino's `Ramp`
/// (`__synthetic__/orchestrator/rust/crates/arduino/src/ramp.rs`). Live and
/// synthetic share no code, so the rule is copied, not shared
/// (real-loop-dia.md, *The command and the Arduino's Run*).
///
/// Rules (each checkable by hand):
/// - The command is the last one received, clamped to ±`top_rate`.
/// - Each tick moves the sent rate towards the command by at most `STEP`, so
///   the sent rate never changes faster than m.
/// - A change within that limit finishes in the tick that takes it; the sent
///   rate never passes the command.
/// - Both start at 0, the cart at rest.
///
/// One difference from the synthetic one: the soft limit may send less than
/// the ramp asks, so the tick's rate is proposed (`next`) and then the rate
/// actually sent is recorded (`sent`), and the next tick ramps from that.
pub(crate) struct Ramp {
    /// The last command after the clamp, pulses/s.
    command: i32,
    /// The rate STEP runs at, pulses/s.
    rate: i32,
}

impl Ramp {
    /// At rest: command 0, sent rate 0.
    pub(crate) const fn new() -> Self {
        Self {
            command: 0,
            rate: 0,
        }
    }

    /// Takes one command, pulses/s (sign = direction), clamped to the top
    /// rate. The sent rate moves only at the next tick.
    pub(crate) fn command(&mut self, rate: i32) {
        self.command = rate.clamp(-TOP_RATE, TOP_RATE);
    }

    /// The rate the ramp asks for at the next tick: towards the command by
    /// at most `STEP`.
    pub(crate) fn next(&self) -> i32 {
        let change = (i64::from(self.command) - i64::from(self.rate)).clamp(-STEP, STEP);
        // Between the sent rate and the command, both i32.
        i32::try_from(i64::from(self.rate) + change).unwrap_or(self.command)
    }

    /// The rate actually sent at this tick: `next`, or less where the soft
    /// limit capped it.
    pub(crate) fn sent(&mut self, rate: i32) {
        self.rate = rate;
    }

    /// The last command after the clamp, pulses/s.
    pub(crate) fn command_rate(&self) -> i32 {
        self.command
    }

    /// The sent rate, pulses/s.
    pub(crate) fn rate(&self) -> i32 {
        self.rate
    }
}
