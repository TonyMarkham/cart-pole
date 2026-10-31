use embedded_hal::{delay::DelayNs, digital::PinState};
use unor4_hal::{
    delay::Delay,
    gpio::{Input, InputPullUp, Output, Pin},
};

use crate::{
    direction::Direction,
    halt::Halt,
    pins::PinStates,
    profile::Profile,
    settings::{
        ACCEL, DIR_SETUP_NS, HOME_SPEED, JOG_PULSES, PULSE_LOW_NS, RELEASE_PULSES, SEEK_SPEED, SEEK_TIMEOUT_MS,
        SETTLE_MS, STOP_CHECK_MS,
    },
    stopwatch::Stopwatch,
    switch::Switch,
    trip::TripFilter,
};

/// The pulses at seek_speed between checks for the page's STOP:
/// seek_speed × stop_check_ms ÷ 1,000, at least 1.
const STOP_EVERY: u32 = {
    let pulses = SEEK_SPEED as u64 * STOP_CHECK_MS as u64 / 1_000;
    if pulses == 0 { 1 } else { pulses as u32 }
};

/// The cart's drive: the driver's inputs, the limit switches, and the
/// signed count of every pulse sent (+ in Plus), which is the only
/// record of where the cart is. Pins as control-design.md wires them;
/// the port and pin numbers are the WiFi board's (unor4-hal
/// `gpio/pins.rs`). The delay is the caller's, shared with the WiFi
/// link.
///
/// Each output reaches the driver through a PNP transistor, so a low pin
/// lights its input and a high one leaves it unlit (control-design.md,
/// *Why the transistors*).
pub struct Axis {
    /// D4 = P106: rests high, low for each pulse.
    step: Pin<'1', 6, Output>,
    /// D5 = P107: low for Minus.
    dir: Pin<'1', 7, Output>,
    /// D6 = P111: low disables the driver, high leaves it enabled.
    enable: Pin<'1', 11, Output>,
    /// D2 = P104: one switch's NC contact, high when pressed.
    d2: Pin<'1', 4, InputPullUp>,
    /// D3 = P105: the other switch, the same way.
    d3: Pin<'1', 5, InputPullUp>,
    /// D8 = P304: the DRV8825's FAULT input, unconnected with the DM542
    /// (it has no fault line), so its pull-up holds it high.
    fault: Pin<'3', 4, InputPullUp>,
    position: i32,
    /// ICLK in Hz, for timing a seek.
    sysclk: u32,
}

impl Axis {
    /// Claims the pins and enables the driver. Every output starts high,
    /// so no input is lit: the driver is enabled and the motor holds
    /// still until the first pulse. The level is set before the pin
    /// starts driving, so the pull-ups' unlit state carries straight on.
    pub fn new(
        step: Pin<'1', 6, Input>,
        dir: Pin<'1', 7, Input>,
        enable: Pin<'1', 11, Input>,
        d2: Pin<'1', 4, Input>,
        d3: Pin<'1', 5, Input>,
        fault: Pin<'3', 4, Input>,
        sysclk: u32,
    ) -> Self {
        Self {
            step: step.into_output_in_state(PinState::High),
            dir: dir.into_output_in_state(PinState::High),
            enable: enable.into_output_in_state(PinState::High),
            // The board's own pull-ups are the switch lines' only ones
            // (control-design.md).
            d2: d2.into_input_pullup(),
            d3: d3.into_input_pullup(),
            fault: fault.into_input_pullup(),
            position: 0,
            sysclk,
        }
    }

    /// Pulses sent since the last `zero`, + in Plus.
    pub fn position(&self) -> i32 {
        self.position
    }

    /// Makes the cart's current place pulse 0.
    pub fn zero(&mut self) {
        self.position = 0;
    }

    /// Sets the count: after a trial's return to the reference switch,
    /// so the steps a failed trial lost do not carry into the next.
    pub fn set_position(&mut self, position: i32) {
        self.position = position;
    }

    /// Switches the driver's outputs off: the motor holds no torque.
    pub fn disable(&mut self) {
        self.enable.set_low();
    }

    /// Switches the driver's outputs on. From off, it waits `settle_ms`
    /// before returning, which also covers the DM542's rule that ENA
    /// leads DIR by 5 µs (manual, section 10, t1).
    pub fn enable(&mut self, delay: &mut Delay) {
        if !self.enable.is_set_high() {
            self.enable.set_high();
            delay.delay_ms(SETTLE_MS);
        }
    }

    /// Every drive pin's level now.
    pub fn pins(&self) -> PinStates {
        PinStates {
            d2: self.d2.is_high(),
            d3: self.d3.is_high(),
            dir: self.dir.is_set_high(),
            step: self.step.is_set_high(),
            enable: self.enable.is_set_high(),
            fault: self.fault.is_high(),
        }
    }

    /// Whether `switch` is pressed. The NC contact opens when the cart
    /// presses it, and so does a broken wire: both read high.
    pub fn pressed(&self, switch: Switch) -> bool {
        match switch {
            Switch::D2 => self.d2.is_high(),
            Switch::D3 => self.d3.is_high(),
        }
    }

    /// Whether `switch` reads pressed at every reading for `trip_us`, so a
    /// spike on its wire is not taken for a pressed switch.
    pub fn pressed_steady(&self, switch: Switch) -> bool {
        let mut clock = Stopwatch::start(self.sysclk);
        let hold = u64::from(crate::settings::TRIP_US) * u64::from(self.sysclk / 1_000_000);
        while clock.cycles() < hold {
            if !self.pressed(switch) {
                return false;
            }
        }
        true
    }

    /// Drives `direction` at seek_speed until either switch trips, and
    /// returns the pulses sent and the switch. It gives up after
    /// `seek_timeout_ms`, timed, not counted, so the limit means the same
    /// at any seek_speed. See `creep` for which switches count and for
    /// STOP.
    pub fn seek(
        &mut self,
        delay: &mut Delay,
        direction: Direction,
        stop: &mut dyn FnMut(&mut Delay) -> bool,
    ) -> Result<(u32, Switch), Halt> {
        match self.creep(delay, direction, Limit::Time(SEEK_TIMEOUT_MS), stop)? {
            (sent, Some(switch)) => Ok((sent, switch)),
            (_, None) => Err(Halt::NoSwitch { direction }),
        }
    }

    /// Drives `direction` at seek_speed for `jog_pulses`, or less if a
    /// switch trips first: returns that switch, if one did.
    pub fn jog(
        &mut self,
        delay: &mut Delay,
        direction: Direction,
        stop: &mut dyn FnMut(&mut Delay) -> bool,
    ) -> Result<Option<Switch>, Halt> {
        self.slow(delay, direction, JOG_PULSES, stop)
    }

    /// Drives `direction` at seek_speed for `pulses`, or less if a switch
    /// trips first: returns that switch, if one did.
    pub fn slow(
        &mut self,
        delay: &mut Delay,
        direction: Direction,
        pulses: u32,
        stop: &mut dyn FnMut(&mut Delay) -> bool,
    ) -> Result<Option<Switch>, Halt> {
        Ok(self.creep(delay, direction, Limit::Pulses(pulses), stop)?.1)
    }

    /// Drives `direction` at seek_speed until `limit`, stopping when a
    /// watched switch trips (`TripFilter`: pressed for `trip_us`). A
    /// switch released when the move
    /// starts is watched at once. One pressed at the start (the end the
    /// cart is leaving) is watched only after it has read released for
    /// `release_pulses` in a row, so its contact bouncing as the cart
    /// leaves it is not a trip. The rate jumps from rest and back, so the
    /// cart stops within a pulse or two of the trip point, and pausing
    /// every `STOP_EVERY` pulses to ask `stop` about the page's STOP is
    /// safe. Returns the pulses sent and the switch that tripped, if any.
    fn creep(
        &mut self,
        delay: &mut Delay,
        direction: Direction,
        limit: Limit,
        stop: &mut dyn FnMut(&mut Delay) -> bool,
    ) -> Result<(u32, Option<Switch>), Halt> {
        let mut clock = Stopwatch::start(self.sysclk);
        let mut trip = TripFilter::new(u64::from(self.sysclk / 1_000_000));
        self.set_direction(delay, direction);
        let period_ns = period_ns(SEEK_SPEED as f32);
        // None: watched. Some(n): pressed at the start, released for n
        // pulses since.
        let mut waiting = Switch::ALL.map(|switch| self.pressed(switch).then_some(0));
        let mut sent = 0;
        loop {
            let now = clock.cycles();
            for (state, switch) in waiting.iter_mut().zip(Switch::ALL) {
                let pressed = self.pressed(switch);
                match state {
                    None if trip.tripped(switch, pressed, now) => return Ok((sent, Some(switch))),
                    None => {}
                    Some(_) if pressed => *state = Some(0),
                    Some(released) if *released + 1 >= RELEASE_PULSES => *state = None,
                    Some(released) => *released += 1,
                }
            }
            let reached = match limit {
                Limit::Pulses(pulses) => sent == pulses,
                Limit::Time(ms) => clock.ms() >= u64::from(ms),
            };
            if reached {
                return Ok((sent, None));
            }
            if sent > 0 && sent % STOP_EVERY == 0 && stop(delay) {
                return Err(Halt::Stopped { position: self.position });
            }
            self.pulse(delay, direction, period_ns)?;
            sent += 1;
        }
    }

    /// Sends exactly `pulses` in `direction`, ramping from `seek_speed` up
    /// to `home_speed` at `accel` and back down, so no change of rate is
    /// larger than the seek's jump from rest. The ramp's rule: each pulse
    /// lasts 1/v s, so it raises the rate by accel × (1/v) pulses/s; the
    /// slow-down starts when the pulses left equal the pulses the
    /// speed-up took.
    ///
    /// A switch tripping on the way (`TripFilter`) halts: the target is
    /// well clear of both ends, so a trip means lost steps or a bad switch.
    /// The switch the move starts on is not watched; it releases as the
    /// cart leaves it. STOP is not checked: pausing mid-ramp would lose
    /// steps, so a STOP pressed during this move takes effect after it.
    pub fn travel(&mut self, delay: &mut Delay, direction: Direction, pulses: u32) -> Result<(), Halt> {
        self.set_direction(delay, direction);
        let start = SEEK_SPEED as f32;
        let top = HOME_SPEED as f32;
        let accel = ACCEL as f32;
        let watch = Switch::ALL.map(|switch| (switch, !self.pressed(switch)));
        let mut clock = Stopwatch::start(self.sysclk);
        let mut trip = TripFilter::new(u64::from(self.sysclk / 1_000_000));
        let mut rate = start;
        let mut speed_up = 0;
        for sent in 0..pulses {
            self.pulse(delay, direction, period_ns(rate))?;
            let now = clock.cycles();
            for (switch, watched) in watch {
                if watched && trip.tripped(switch, self.pressed(switch), now) {
                    return Err(Halt::SwitchOnTheWay {
                        switch,
                        position: self.position,
                    });
                }
            }
            let left_to_send = pulses - sent - 1;
            if left_to_send <= speed_up {
                rate = (rate - accel / rate).max(start);
            } else if rate < top {
                rate = (rate + accel / rate).min(top);
                speed_up += 1;
            }
        }
        Ok(())
    }

    /// Plays a trial's rate plan from where the cart is: at each moment
    /// the rate the plan gives, + in Plus. Pulses are timed on the cycle
    /// counter against the plan's clock, not by delays between them, so
    /// the loop's own work does not slow the rate, and a jump lands where
    /// the plan puts it, within one pulse period (21 µs at 48,000
    /// pulses/s; while the rate is 0 the plan is checked every 10 µs). A
    /// switch tripping (`TripFilter`) halts. STOP is not checked: a pause would
    /// itself be a jump.
    pub fn run_profile(&mut self, profile: &Profile) -> Result<(), Halt> {
        let mut clock = Stopwatch::start(self.sysclk);
        let per_us = u64::from(self.sysclk / 1_000_000);
        let end = profile.duration_us() * per_us;
        let watch = Switch::ALL.map(|switch| (switch, !self.pressed(switch)));
        let mut trip = TripFilter::new(per_us);
        let mut next = 0;
        let mut heading = None;
        loop {
            let now = clock.cycles();
            if now >= end {
                return Ok(());
            }
            if now < next {
                continue;
            }
            let rate = profile.rate_at(now / per_us);
            if rate == 0 {
                next = now + 10 * per_us;
                continue;
            }
            let direction = if rate > 0 { Direction::Plus } else { Direction::Minus };
            if heading != Some(direction) {
                self.set_dir_pin(direction);
                clock.spin_ns(DIR_SETUP_NS);
                heading = Some(direction);
            }
            let at = clock.cycles();
            self.step.set_low();
            clock.spin_ns(PULSE_LOW_NS);
            self.step.set_high();
            self.position += direction.sign();
            next = at + u64::from(self.sysclk) / u64::from(rate.unsigned_abs());
            if self.fault.is_low() {
                return Err(Halt::DriverFault { position: self.position });
            }
            let now = clock.cycles();
            for (switch, watched) in watch {
                if watched && trip.tripped(switch, self.pressed(switch), now) {
                    return Err(Halt::SwitchOnTheWay { switch, position: self.position });
                }
            }
        }
    }

    fn set_direction(&mut self, delay: &mut Delay, direction: Direction) {
        self.set_dir_pin(direction);
        delay.delay_ns(DIR_SETUP_NS);
    }

    fn set_dir_pin(&mut self, direction: Direction) {
        if direction.dir_high() {
            self.dir.set_high();
        } else {
            self.dir.set_low();
        }
    }

    /// One STEP pulse lasting `period_ns` (D4 low for `pulse_low_ns`),
    /// counted, then D8 checked: low
    /// means something is wired to it. The loop's own work adds to the
    /// period, so the real rate is a little under the asked one, never
    /// over it.
    fn pulse(&mut self, delay: &mut Delay, direction: Direction, period_ns: u32) -> Result<(), Halt> {
        self.step.set_low();
        delay.delay_ns(PULSE_LOW_NS);
        self.step.set_high();
        delay.delay_ns(period_ns.saturating_sub(PULSE_LOW_NS));
        self.position += direction.sign();
        if self.fault.is_low() {
            return Err(Halt::DriverFault {
                position: self.position,
            });
        }
        Ok(())
    }
}

/// Where a slow move ends if no switch trips first.
#[derive(Clone, Copy, Debug)]
enum Limit {
    /// After this many pulses: a jog.
    Pulses(u32),
    /// After this many ms: a seek giving up.
    Time(u32),
}

/// One pulse's period at `rate` pulses/s, ns. The build script holds
/// every rate to 1..=200,000, so the period is 5,000 ns to 1 s.
fn period_ns(rate: f32) -> u32 {
    (1.0e9 / rate) as u32
}
