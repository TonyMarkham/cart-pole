use crate::{
    direction::Direction,
    end::End,
    error::{AppError, result::AppResult},
    limit::Limit,
    moved::Moved,
    pins::Pins,
    seek::{creep, seek},
    settings::{CENTRE_SPEED_MAX, IDLER_END, MOTOR_END, SEEK_PERIOD_NS},
    switch::Switch,
};

use unor4_hal::{delay::Delay, uart::UartIrq};

/// What the board knows about the rail, which the stepper cannot report. It is
/// lost at every reset, so both ends must be found again before a centring.
pub struct Rail {
    /// Pulses from one switch's trip to the other's.
    span: Option<u32>,
    /// The switch the last move tripped and stopped on; None once the cart has
    /// left it.
    resting_on: Option<Switch>,
}

impl Rail {
    pub const fn new() -> Self {
        Self {
            span: None,
            resting_on: None,
        }
    }

    /// Moves to `end`, and keeps what the move found out. A switch counts only
    /// when this board saw it trip during a move: one that merely reads pressed
    /// may be a broken wire, not the cart. So the span needs the cart to start
    /// on a trip the board saw, still pressed, and to trip the other end.
    pub fn seek(
        &mut self,
        at: &'static UartIrq<ra4m1::SCI1>,
        pins: &mut Pins,
        delay: &mut Delay,
        end: End,
    ) -> AppResult<Moved> {
        let left_trip = self.resting_on.filter(|&last| pins.pressed_steady(last));
        match seek(at, pins, delay, end) {
            Ok(Moved::Tripped { switch, pulses }) if switch == end.switch => {
                if left_trip.is_some_and(|last| last != switch) {
                    self.span = Some(pulses);
                }
                self.resting_on = Some(switch);
                Ok(Moved::Tripped { switch, pulses })
            }
            // The wrong switch, or none where one should be: something is
            // wrong, so the ends count as not found until both are found again.
            Ok(Moved::Tripped { switch, pulses }) => {
                self.forget();
                Err(AppError::wrong_switch(switch, pulses))
            }
            Err(error @ AppError::TimedOut { .. }) => {
                self.forget();
                Err(error)
            }
            // A stop loses no steps, so the span stands, but the cart may have
            // left the switch it rested on.
            Err(error @ AppError::Stopped { .. }) => {
                self.resting_on = None;
                Err(error)
            }
            other => other,
        }
    }

    /// Moves half the span from the switch the cart rests on: the centre is
    /// halfway between the two trip points. `speed` is pulses/s; None is
    /// `seek_speed`. The count is known, so no time limit applies, and a
    /// switch tripping on the way ends the move: it means lost steps.
    pub fn centre(
        &mut self,
        at: &'static UartIrq<ra4m1::SCI1>,
        pins: &mut Pins,
        delay: &mut Delay,
        speed: Option<u32>,
    ) -> AppResult<Moved> {
        let period_ns = match speed {
            None => SEEK_PERIOD_NS,
            Some(speed) if (1..=CENTRE_SPEED_MAX).contains(&speed) => 1_000_000_000 / speed,
            Some(_) => return Err(AppError::bad_speed()),
        };
        let (Some(span), Some(switch)) = (self.span, self.resting_on) else {
            return Err(AppError::not_ready());
        };
        let [d2, d3] = Switch::ALL.map(|switch| pins.pressed_steady(switch));
        if d2 && d3 {
            return Err(AppError::refused());
        }
        // Not on it any more: moved by hand, so half the span would not be the centre.
        if !pins.pressed_steady(switch) {
            self.resting_on = None;
            return Err(AppError::not_ready());
        }

        self.resting_on = None;
        let moved = creep(
            at,
            pins,
            delay,
            away_from(switch),
            period_ns,
            Limit::Pulses(span / 2),
        )?;
        if let Moved::Tripped { switch, pulses } = moved {
            self.resting_on = Some(switch);
            return Err(AppError::switch_on_the_way(switch, pulses));
        }
        Ok(moved)
    }

    fn forget(&mut self) {
        self.span = None;
        self.resting_on = None;
    }
}

/// The direction that leaves `switch`: the way to the other end.
fn away_from(switch: Switch) -> Direction {
    if switch == IDLER_END.switch {
        MOTOR_END.direction
    } else {
        IDLER_END.direction
    }
}
