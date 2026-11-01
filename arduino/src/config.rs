/// The drive train and the limits the firmware enforces. All lengths are
/// whole micrometers so every derived pulse count is integer arithmetic
/// that can be checked by hand; the board has no floating-point unit to
/// rely on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Motor full steps per revolution: the trainer's settings.toml
    /// `[drive] full_steps_per_rev`.
    pub full_steps_per_rev: u32,
    /// The driver's microstep setting, as its denominator (1/32 is 32):
    /// `[drive] microsteps`.
    pub microsteps: u32,
    /// Belt travel per pulley revolution, µm: `[drive] belt_um_per_rev`.
    pub belt_um_per_rev: u32,
    /// The cart top speed, µm/s: the MJCF actuator's `ctrlrange`, which
    /// the loop reads from MuJoCo. Commands above it are clamped to it.
    pub max_speed_um_per_s: u32,
    /// m, the jump limit, pulses/s²: the fastest the sent rate may change,
    /// measured on the rig (`jump-limit-results.md`): `[drive] jump_limit`.
    pub jump_limit: u32,
}

/// One second in the time unit `ramp_limit` takes.
const MICROS_PER_SECOND: u64 = 1_000_000;

impl Config {
    /// Step pulses per pulley revolution.
    pub const fn pulses_per_rev(&self) -> u32 {
        self.full_steps_per_rev * self.microsteps
    }

    /// The top pulse rate, pulses/s (rounded down, so the cart never
    /// exceeds `max_speed_um_per_s`).
    pub const fn max_rate(&self) -> i32 {
        (self.max_speed_um_per_s as u64 * self.pulses_per_rev() as u64
            / self.belt_um_per_rev as u64) as i32
    }

    /// The most the sent rate may change in a tick of `micros` µs,
    /// pulses/s: m × the tick, rounded down, so the ramp never exceeds m
    /// (1,210,937 × 2 ms = 2,421.874 → 2,421).
    pub const fn ramp_limit(&self, micros: u32) -> u64 {
        self.jump_limit as u64 * micros as u64 / MICROS_PER_SECOND
    }

    /// A commanded pulse rate clamped to ±`max_rate`, the one clamp the
    /// board and the synthetic build share.
    pub const fn clamp_rate(&self, rate: i32) -> i32 {
        let max = self.max_rate();
        if rate > max {
            max
        } else if rate < -max {
            -max
        } else {
            rate
        }
    }
}
