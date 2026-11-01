//! The slow moves: pulses at a steady rate until a limit switch trips or the
//! limit is reached, then the cart stops and the driver holds it. A fixed rule,
//! with no choices made on the way: whichever switch trips ends the move, and
//! the answer names it, so a wrong guess in `calibration.toml` shows there.

use crate::{
    direction::Direction,
    end::End,
    error::{AppError, result::AppResult},
    limit::Limit,
    moved::Moved,
    pins::Pins,
    settings::{RELEASE_PULSES, SEEK_PERIOD_NS, SEEK_TIMEOUT_MS},
    stopwatch::Stopwatch,
    switch::Switch,
    trip::TripFilter,
};

use unor4_hal::delay::Delay;

/// Moves to `end` at `seek_speed`.
pub fn seek(pins: &mut Pins, delay: &mut Delay, end: End) -> AppResult<Moved> {
    let [d2, d3] = Switch::ALL.map(|switch| pins.pressed_steady(switch));
    if d2 && d3 {
        return Err(AppError::refused());
    }
    if pins.pressed_steady(end.switch) {
        return Err(AppError::already_there(end.switch));
    }
    creep(
        pins,
        delay,
        end.direction,
        SEEK_PERIOD_NS,
        Limit::Time(SEEK_TIMEOUT_MS),
    )
}

/// Pulses `direction`, one every `period_ns`, until a switch trips or `limit`
/// is reached. A switch released when the move starts is watched at once. One
/// pressed at the start, the end the cart is leaving, is watched only after it
/// has read released for `release_pulses` in a row, so its contact bouncing as
/// the cart leaves is not a trip.
pub fn creep(
    pins: &mut Pins,
    delay: &mut Delay,
    direction: Direction,
    period_ns: u32,
    limit: Limit,
) -> AppResult<Moved> {
    pins.set_direction(delay, direction);
    let sysclk = pins.sysclk();
    let mut clock = Stopwatch::start(sysclk);
    let mut trip = TripFilter::new(u64::from(sysclk / 1_000_000));
    // None: watched. Some(n): pressed at the start, released for n pulses since.
    let mut waiting: [Option<u32>; 2] = Switch::ALL.map(|switch| pins.pressed(switch).then_some(0));
    let mut pulses = 0;
    loop {
        let now = clock.cycles();
        for (state, switch) in waiting.iter_mut().zip(Switch::ALL) {
            let pressed = pins.pressed(switch);
            match state {
                None if trip.tripped(switch, pressed, now) => {
                    return Ok(Moved::Tripped { switch, pulses });
                }
                None => {}
                Some(_) if pressed => *state = Some(0),
                Some(released) if *released + 1 >= RELEASE_PULSES => *state = None,
                Some(released) => *released += 1,
            }
        }
        match limit {
            Limit::Pulses(count) if pulses >= count => return Ok(Moved::Centred { pulses }),
            Limit::Time(ms) if clock.ms() >= u64::from(ms) => {
                return Err(AppError::timed_out(pulses));
            }
            _ => {}
        }
        pins.pulse(delay, period_ns);
        pulses += 1;
    }
}
