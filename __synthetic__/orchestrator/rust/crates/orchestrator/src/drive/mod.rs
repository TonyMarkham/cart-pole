use anyhow::{Result, bail};
use arduino::{Config, Direction, Dm542Signals};

pub(crate) fn drive_config(
    full_steps_per_rev: u32,
    microsteps: u32,
    belt_um_per_rev: u32,
    jump_limit: u32,
    max_speed: f64,
) -> Result<Config> {
    if full_steps_per_rev.checked_mul(microsteps).is_none() {
        bail!(
            "--full-steps-per-rev {full_steps_per_rev} × --microsteps {microsteps} does not fit a u32"
        );
    }
    let um_per_s = (max_speed * 1e6).round();
    if !(1.0..=f64::from(u32::MAX)).contains(&um_per_s) {
        bail!(
            "the cart top speed {max_speed} m/s is not a whole number of µm/s from 1 to {}",
            u32::MAX
        );
    }
    let config = Config {
        full_steps_per_rev,
        microsteps,
        belt_um_per_rev,
        max_speed_um_per_s: um_per_s as u32,
        jump_limit,
    };
    let rate = u64::from(config.max_speed_um_per_s) * u64::from(config.pulses_per_rev())
        / u64::from(belt_um_per_rev);
    if rate < 1 || rate > i32::MAX as u64 {
        bail!(
            "the top pulse rate {rate} pulses/s (cart top speed {max_speed} m/s ÷ metres per pulse) is not from 1 to {}",
            i32::MAX
        );
    }
    Ok(config)
}

pub(crate) fn ramp_tick(config: &Config, timestep: f64) -> Result<u32> {
    let micros = (timestep * 1e6).round();
    if !(1.0..=f64::from(u32::MAX)).contains(&micros) {
        bail!(
            "the timestep {timestep} s is not a whole number of µs from 1 to {}",
            u32::MAX
        );
    }
    let tick = micros as u32;
    if config.ramp_limit(tick) < 1 {
        bail!(
            "--jump-limit {} pulses/s² × the {tick} µs timestep rounds to 0 pulses/s a tick: the cart could never move",
            config.jump_limit
        );
    }
    Ok(tick)
}

pub(crate) fn meters_per_pulse(config: &Config) -> f64 {
    f64::from(config.belt_um_per_rev) / 1e6 / f64::from(config.pulses_per_rev())
}

pub(crate) fn cart_velocity(signals: Dm542Signals, meters_per_pulse: f64) -> f64 {
    let speed = f64::from(signals.step_rate) * meters_per_pulse;
    match signals.dir {
        Direction::Positive => speed,
        Direction::Negative => -speed,
    }
}
