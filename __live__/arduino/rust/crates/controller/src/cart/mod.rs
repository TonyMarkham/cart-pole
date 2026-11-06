pub(crate) mod calibration_move;
pub(crate) mod centre_end;
pub(crate) mod centre_speed;
pub(crate) mod error;
pub(crate) mod move_end;
pub(crate) mod move_timer;
pub(crate) mod move_wake;
pub(crate) mod rail;
pub(crate) mod rail_end;
pub(crate) mod seek_end;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    cart::{
        centre_end::CentreEnd,
        centre_speed::CentreSpeed,
        error::{move_error::MoveError, move_result::MoveResult},
        move_end::MoveEnd,
        move_wake::MoveWake,
        rail::Rail,
        rail_end::RailEnd,
        seek_end::SeekEnd,
    },
    config::calibration::{RELEASE_PULSES, SEEK_SPEED, SEEK_TIMEOUT_MS},
    constants::MICROS_PER_SECOND,
    sleep,
    wifi::{deadline::Deadline, message::Message as WifiMessage, udp_socket::UdpSocket},
};

use core::time::Duration;
use unor4_hal::{
    gpio::{
        Switch,
        contact::Nc,
        pin_map::{D2, D3},
    },
    step::StepGenerator,
    wake::WakeTimer,
};

/// The parts that move the cart and tell where it is: the step generator, the
/// limit switch at each end, the move timer, which times a move's own limits,
/// and what the board knows of where the cart is.
pub(crate) struct Cart {
    stepper: StepGenerator,
    idler: Switch<D3, Nc>,
    motor: Switch<D2, Nc>,
    timer: WakeTimer,
    /// Where the cart is, in pulses from where it stood at start-up, positive
    /// towards the motor's end. The step generator counts every pulse but not
    /// its direction, so each move adds its own pulses with its rate's sign.
    position: i64,
    rail: Rail,
}

impl Cart {
    /// Takes the parts, and turns the motor driver on: from here on it holds
    /// the cart in place between moves, so its position stays known. Its ENA
    /// lead (5 µs) has long passed by the first move, which can only come
    /// after the WiFi join. A move armed with a pulse count wakes the board
    /// when the count is reached.
    pub(crate) fn new(
        mut stepper: StepGenerator,
        idler: Switch<D3, Nc>,
        motor: Switch<D2, Nc>,
        timer: WakeTimer,
    ) -> Self {
        stepper.enable();
        stepper.set_count_wake(true);
        Self {
            stepper,
            idler,
            motor,
            timer,
            position: 0,
            rail: Rail::new(),
        }
    }

    /// Seeks the end `toward`: runs at `seek_speed` until that end's switch
    /// trips, then stops. Returns how the seek ended and the pulses it sent.
    /// Reaching the end records it in the rail. Reaching the other end, or
    /// running out of time, forgets the rail, since the count may be wrong.
    pub(crate) fn seek(
        &mut self,
        toward: RailEnd,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<(SeekEnd, u32)> {
        let at_end = self.pressed(toward);
        let on_other = self.pressed(toward.other());
        if at_end && on_other {
            return Ok((SeekEnd::Refused, 0));
        }
        if at_end {
            return Ok((SeekEnd::AlreadyThere, 0));
        }

        let timeout = Duration::from_millis(u64::from(SEEK_TIMEOUT_MS));
        let leaving = on_other.then_some(toward.other());
        let (end, pulses) = self.drive(
            toward.rate(SEEK_SPEED),
            leaving,
            Some(timeout),
            None,
            udp,
            wake,
        )?;
        let end = match end {
            MoveEnd::Switch(end) if end == toward => {
                self.rail.record(toward, self.position);
                SeekEnd::Arrived
            }
            MoveEnd::Switch(_) => {
                self.rail.forget();
                SeekEnd::OtherEnd
            }
            MoveEnd::TimedOut => {
                self.rail.forget();
                SeekEnd::TimedOut
            }
            // A seek arms no pulse count, so only a `Stop` is left.
            MoveEnd::Stopped | MoveEnd::Count => SeekEnd::Stopped,
        };
        Ok((end, pulses))
    }

    /// Moves to the centre of the rail at `speed`, from wherever the cart is,
    /// and stops there by pulse count. Returns how the move ended and the
    /// pulses it sent. A switch tripping on the way forgets the rail: the cart
    /// was not where the count said.
    pub(crate) fn centre(
        &mut self,
        speed: CentreSpeed,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<(CentreEnd, u32)> {
        let Some(speed) = speed.speed() else {
            return Ok((CentreEnd::BadSpeed, 0));
        };
        let Some(centre) = self.rail.centre() else {
            return Ok((CentreEnd::NotReady, 0));
        };
        let on_idler = self.pressed(RailEnd::Idler);
        let on_motor = self.pressed(RailEnd::Motor);
        if on_idler && on_motor {
            return Ok((CentreEnd::Refused, 0));
        }

        let distance = centre - self.position;
        // At most the span, so it always fits; a rail that says otherwise is
        // wrong, and is forgotten.
        let Ok(count) = u32::try_from(distance.unsigned_abs()) else {
            self.rail.forget();
            return Ok((CentreEnd::NotReady, 0));
        };
        if count == 0 {
            return Ok((CentreEnd::Centred, 0));
        }

        let toward = if distance > 0 {
            RailEnd::Motor
        } else {
            RailEnd::Idler
        };
        let leaving = if on_idler {
            Some(RailEnd::Idler)
        } else if on_motor {
            Some(RailEnd::Motor)
        } else {
            None
        };
        let (end, pulses) =
            self.drive(toward.rate(speed), leaving, None, Some(count), udp, wake)?;
        let end = match end {
            MoveEnd::Count => CentreEnd::Centred,
            MoveEnd::Switch(end) => {
                self.rail.forget();
                CentreEnd::Tripped(end)
            }
            // The move has no time limit, so only a `Stop` is left.
            MoveEnd::Stopped | MoveEnd::TimedOut => CentreEnd::Stopped,
        };
        Ok((end, pulses))
    }

    /// Runs the cart at `rate` until something ends the move, then stops it,
    /// adds the pulses it sent to `position`, and returns what ended it and
    /// the pulses. It sleeps the whole way, woken by a switch, a datagram, the
    /// move timer or the pulse count. Only `Stop` is taken: any other datagram
    /// is dropped unanswered (real-open-issues.md, issue 5).
    ///
    /// `leaving`: a switch the cart starts on, whose presses count only once
    /// the cart has moved `release_pulses` off it. `limit`: the longest the
    /// move may run, or None. `count`: the pulses after which the hardware
    /// stops the move, or None.
    fn drive(
        &mut self,
        rate: i32,
        leaving: Option<RailEnd>,
        limit: Option<Duration>,
        count: Option<u32>,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<(MoveEnd, u32)> {
        // Only a press during the move counts.
        let _ = self.tripped(RailEnd::Idler);
        let _ = self.tripped(RailEnd::Motor);

        let start = self.stepper.pulses();
        let ended = self
            .start(rate, start, count)
            .and_then(|()| self.until_move_ends(rate, leaving, limit, count.is_some(), udp, wake));
        // Stopped whatever happened, an error included: no pulse is cut, and
        // the last one is counted before `stop` returns (unor4-hal
        // `step/step_generator.rs`, `stop`). A count stop has stopped it
        // already, and `stop` then does nothing.
        self.stepper.stop();
        self.timer.cancel();
        let pulses = self.stepper.pulses().wrapping_sub(start);
        self.position += if rate < 0 {
            -i64::from(pulses)
        } else {
            i64::from(pulses)
        };
        Ok((ended?, pulses))
    }

    /// Arms the pulse count, if any, from `start`, then starts the pulses.
    fn start(&mut self, rate: i32, start: u32, count: Option<u32>) -> MoveResult<()> {
        if let Some(count) = count {
            self.stepper.stop_at(start.wrapping_add(count))?;
        }
        self.stepper.run(rate)?;
        Ok(())
    }

    /// Sleeps until the move at `rate` is over, and says what ended it. A
    /// switch the cart is `leaving` is checked once the cart should be off
    /// it: still pressed, the cart is driving into it. The move timer times
    /// that check first, then what is left of `limit`.
    fn until_move_ends(
        &mut self,
        rate: i32,
        mut leaving: Option<RailEnd>,
        limit: Option<Duration>,
        counted: bool,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<MoveEnd> {
        let release = release_time(rate);
        let first = if leaving.is_some() {
            Some(release)
        } else {
            limit
        };
        let mut deadline = self.deadline(first)?;
        loop {
            let woke = sleep::until(|| {
                self.look(leaving, counted, &mut deadline, udp, wake)
                    .transpose()
            })?;
            match woke {
                MoveWake::Ended(end) => return Ok(end),
                MoveWake::LeftSwitch => {
                    if let Some(end) = leaving.take() {
                        if self.pressed(end) {
                            return Ok(MoveEnd::Switch(end));
                        }
                        // Its bounces as the cart left it don't count.
                        let _ = self.tripped(end);
                    }
                    deadline = self.deadline(limit.map(|limit| limit.saturating_sub(release)))?;
                }
            }
        }
    }

    /// One look during a move, without sleeping: what has happened, or None
    /// if nothing the move acts on.
    fn look(
        &mut self,
        leaving: Option<RailEnd>,
        counted: bool,
        deadline: &mut Option<Deadline>,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<Option<MoveWake>> {
        if counted && self.stepper.stopped_at_count() {
            return Ok(Some(MoveWake::Ended(MoveEnd::Count)));
        }
        for end in [RailEnd::Idler, RailEnd::Motor] {
            if leaving != Some(end) && self.tripped(end) {
                return Ok(Some(MoveWake::Ended(MoveEnd::Switch(end))));
            }
        }
        while let Some(datagram) = udp.look(wake).map_err(MoveError::WakeTimer)? {
            if WifiMessage::parse(&datagram) == Some(WifiMessage::Stop) {
                return Ok(Some(MoveWake::Ended(MoveEnd::Stopped)));
            }
        }
        if let Some(deadline) = deadline.as_mut()
            && deadline
                .passed(&mut self.timer)
                .map_err(MoveError::MoveTimer)?
        {
            return Ok(Some(if leaving.is_some() {
                MoveWake::LeftSwitch
            } else {
                MoveWake::Ended(MoveEnd::TimedOut)
            }));
        }
        Ok(None)
    }

    /// Starts `wait` on the move timer, or nothing for None.
    fn deadline(&mut self, wait: Option<Duration>) -> MoveResult<Option<Deadline>> {
        wait.map(|wait| Deadline::start(&mut self.timer, wait))
            .transpose()
            .map_err(MoveError::MoveTimer)
    }

    /// Whether `end`'s switch reads pressed now.
    fn pressed(&self, end: RailEnd) -> bool {
        match end {
            RailEnd::Idler => self.idler.is_active(),
            RailEnd::Motor => self.motor.is_active(),
        }
    }

    /// Whether `end`'s switch was pressed since the last call, even if it is
    /// released again; it then forgets that press.
    fn tripped(&mut self, end: RailEnd) -> bool {
        match end {
            RailEnd::Idler => self.idler.activated(),
            RailEnd::Motor => self.motor.activated(),
        }
    }
}

/// How long `release_pulses` takes at `rate`, rounded up so the check never
/// comes early. A move runs at one rate, start to stop.
fn release_time(rate: i32) -> Duration {
    let speed = u64::from(rate.unsigned_abs()).max(1);
    Duration::from_micros((u64::from(RELEASE_PULSES) * MICROS_PER_SECOND).div_ceil(speed))
}
