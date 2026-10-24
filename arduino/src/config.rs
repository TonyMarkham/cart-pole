/// The drive train and the limits the firmware enforces. All lengths are
/// whole micrometers so every derived pulse count is integer arithmetic
/// that can be checked by hand; the board has no floating-point unit to
/// rely on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Motor full steps per revolution (a 1.8° motor: 200).
    pub full_steps_per_rev: u32,
    /// The driver's microstep setting (1/32 → 32).
    pub microsteps: u32,
    /// Belt travel per pulley revolution, µm (48T GT2 at 2 mm pitch:
    /// 96,000).
    pub belt_um_per_rev: u32,
    /// The usable travel either side of the rail's center, µm. The
    /// firmware emits no pulse that would take the cart past it.
    pub travel_um: u32,
    /// The motor's top speed, µm/s. Commands above it are clamped to it.
    pub max_speed_um_per_s: u32,
}

/// The rig (synthetic-arduino-plan.md, mujoco-plan.md *The actuator's values*):
/// 23HS32-4004S (1.8°), DRV8825, 48T GT2 pulley, 900 mm usable rail.
/// `microsteps` is a placeholder until the driver's setting is chosen
/// (1/32 is the DRV8825's finest); `max_speed_um_per_s` is the 24 V
/// corner-speed estimate, the MJCF's `ctrlrange`.
///
/// At these values: 6,400 pulses per revolution, 15 µm per pulse
/// (96,000 / 6,400), ±30,000 pulses of travel (450,000 / 15), and a top
/// rate of 45,333 pulses/s (680,000 / 15, rounded down).
pub const RIG: Config = Config {
    full_steps_per_rev: 200,
    microsteps: 32,
    belt_um_per_rev: 96_000,
    travel_um: 450_000,
    max_speed_um_per_s: 680_000,
};

impl Config {
    /// Step pulses per pulley revolution.
    pub const fn pulses_per_rev(&self) -> u32 {
        self.full_steps_per_rev * self.microsteps
    }

    /// The travel either side of center, in pulses (rounded down, so the
    /// envelope never reaches past `travel_um`).
    pub const fn travel_pulses(&self) -> i32 {
        (self.travel_um as u64 * self.pulses_per_rev() as u64 / self.belt_um_per_rev as u64) as i32
    }

    /// The top pulse rate, pulses/s (rounded down, so the cart never
    /// exceeds `max_speed_um_per_s`).
    pub const fn max_rate(&self) -> i32 {
        (self.max_speed_um_per_s as u64 * self.pulses_per_rev() as u64 / self.belt_um_per_rev as u64) as i32
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
