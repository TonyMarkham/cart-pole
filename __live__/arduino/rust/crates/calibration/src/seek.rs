//! The slow moves: pulses at a steady rate until a limit switch trips or the
//! limit is reached or a `Stop` datagram arrives, then the cart stops and the
//! driver holds it. A fixed rule, with no choices made on the way: whichever
//! switch trips ends the move, and the answer names it, so a wrong guess in
//! `calibration.toml` shows there.

use crate::{
    direction::Direction,
    end::End,
    error::{AppError, result::AppResult},
    limit::Limit,
    moved::Moved,
    pins::Pins,
    settings::{RELEASE_PULSES, SEEK_SPEED, SEEK_TIMEOUT_MS},
    sleep,
    stop_listener::StopListener,
    stopwatch::Stopwatch,
    switch::Switch,
    trip::TripFilter,
};

use unor4_hal::uart::UartIrq;

/// Moves to `end` at `seek_speed`.
pub fn seek(at: &'static UartIrq<ra4m1::SCI1>, pins: &mut Pins, end: End) -> AppResult<Moved> {
    let d2 = pins.pressed_steady(Switch::D2)?;
    let d3 = pins.pressed_steady(Switch::D3)?;
    if d2 && d3 {
        return Err(AppError::refused());
    }
    if pins.pressed_steady(end.switch)? {
        return Err(AppError::already_there(end.switch));
    }
    creep(
        at,
        pins,
        end.direction,
        SEEK_SPEED,
        Limit::Time(SEEK_TIMEOUT_MS),
    )
}

/// Runs `direction` at `speed` pulses/s, from rest and with no ramp, until a
/// switch trips, `limit` is reached or a `Stop` datagram arrives. The step
/// generator makes and counts the pulses in hardware, so the board sleeps
/// through the move. It wakes at each 2 ms tick, at a press on either switch,
/// when the wake timer ends a pressed switch's `trip_us`, at the count stop
/// and at each byte from the WiFi chip, and looks only then.
/// A switch released when the move starts is watched at once. One pressed at
/// the start, the end the cart is leaving, is watched only after no press of
/// it has been seen for `release_pulses`, so its contact bouncing as the cart
/// leaves is not a trip. A pulse limit is a count stop: the generator stops
/// in hardware after exactly that many pulses, and the answer gives the count
/// the cart really made.
pub fn creep(
    at: &'static UartIrq<ra4m1::SCI1>,
    pins: &mut Pins,
    direction: Direction,
    speed: u32,
    limit: Limit,
) -> AppResult<Moved> {
    let count = match limit {
        // Already there: a count stop must be at least 1 ahead.
        Limit::Pulses(0) => return Ok(Moved::Centred { pulses: 0 }),
        Limit::Pulses(count) => Some(count),
        Limit::Time(_) => None,
    };
    let sysclk = pins.sysclk();
    let mut trip = TripFilter::new(u64::from(sysclk / 1_000_000));
    // None: watched. Some(n): pressed at the start; n is the pulse count at
    // the latest press seen.
    let mut waiting: [Option<u32>; 2] = Switch::ALL.map(|switch| pins.pressed(switch).then_some(0));
    pins.clear_edges();
    let mut stop = StopListener::new(at);
    let start = pins.start(direction, speed, count)?;
    let mut clock = Stopwatch::start(sysclk);
    // The `trip.due()` the wake timer was last started for; None: not armed.
    let mut armed: Option<u64> = None;
    loop {
        let (wake, heard) = sleep::until(|| {
            let wake = pins.woke();
            let heard = stop.heard();
            (heard || wake.any()).then_some((wake, heard))
        });
        if heard {
            return Err(AppError::stopped(pins.halt(start)));
        }
        let pulses = pins.moved(start);
        let now = clock.cycles();
        for ((state, switch), edge) in waiting.iter_mut().zip(Switch::ALL).zip(wake.edges) {
            let pressed = pins.pressed(switch);
            match state {
                None if trip.tripped(switch, pressed, edge, now) => {
                    return Ok(Moved::Tripped {
                        switch,
                        pulses: pins.halt(start),
                    });
                }
                None => {}
                Some(since) if pressed || edge => *since = pulses,
                Some(since) if pulses.wrapping_sub(*since) >= RELEASE_PULSES => *state = None,
                Some(_) => {}
            }
        }
        if wake.counted {
            return Ok(Moved::Centred {
                pulses: pins.halt(start),
            });
        }
        if let Limit::Time(ms) = limit
            && clock.ms() >= u64::from(ms)
        {
            return Err(AppError::timed_out(pins.halt(start)));
        }
        // One wake timer serves both switches: it is started for the earlier
        // `due`, again whenever that changes, and again after each end, which
        // lands on or after `due` (the timer rounds up) unless the two clocks
        // part by a count; then it starts once more for what is left.
        let due = trip.due();
        if wake.timer || due != armed {
            match due {
                Some(due) => {
                    if let Err(error) = pins.wake_in(due.saturating_sub(now)) {
                        pins.halt(start);
                        return Err(error);
                    }
                }
                None => pins.cancel_wake(),
            }
            armed = due;
        }
    }
}
