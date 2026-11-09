pub(crate) mod calibration_move;
pub(crate) mod centre_end;
pub(crate) mod centre_speed;
pub(crate) mod centre_wake;
pub(crate) mod error;
pub(crate) mod move_end;
pub(crate) mod move_timer;
pub(crate) mod move_wake;
pub(crate) mod moving_end;
pub(crate) mod moving_wake;
pub(crate) mod rail;
pub(crate) mod rail_end;
pub(crate) mod ramp;
pub(crate) mod seek_end;
pub(crate) mod stop_points;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    cart::{
        centre_end::CentreEnd,
        centre_speed::CentreSpeed,
        centre_wake::CentreWake,
        error::{move_error::MoveError, move_result::MoveResult},
        move_end::MoveEnd,
        move_wake::MoveWake,
        moving_end::MovingEnd,
        moving_wake::MovingWake,
        rail::Rail,
        rail_end::RailEnd,
        ramp::Ramp,
        seek_end::SeekEnd,
        stop_points::StopPoints,
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
    step::{StepError, StepGenerator},
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
        let (end, pulses) = self.drive(toward.rate(SEEK_SPEED), leaving, timeout, udp, wake)?;
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
            MoveEnd::BadSign => SeekEnd::Fault,
            MoveEnd::Stopped => SeekEnd::Stopped,
        };
        Ok((end, self.pulses_for(end == SeekEnd::Fault, pulses)))
    }

    /// Moves to the centre of the rail at `speed`, from wherever the cart is,
    /// ramped both ways so no step is lost. Every 2 ms tick the ramp takes one
    /// step towards `speed`, and the soft limit, with the centre as its one
    /// stop point, brakes the cart to rest on it. The hardware count stop,
    /// armed at the centre, ends the move there, so the cart never passes it.
    /// Returns how the move ended and the pulses it sent. A switch tripping on
    /// the way forgets the rail: the cart was not where the count said. A
    /// `Stop` brakes the cart to rest as a 0 command does; a fault stops it in
    /// one jump.
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
        let mut ramp = Ramp::new();
        ramp.command(toward.rate(speed));

        // Only a press during the move counts.
        let _ = self.tripped(RailEnd::Idler);
        let _ = self.tripped(RailEnd::Motor);

        let base = self.start_ticks();
        let start = self.position;
        let ended = self
            .stepper
            .stop_at(self.stepper.pulses().wrapping_add(count))
            .map_err(MoveError::from)
            .and_then(|()| {
                self.until_centred(&mut ramp, StopPoints::at(centre), leaving, base, udp, wake)
            });
        self.stop_ticks(base, start);

        let end = ended?;
        if let CentreEnd::Tripped(_) = end {
            self.rail.forget();
        }
        // At most `count`: the count stop ends the move there.
        let pulses = u32::try_from((self.position - start).unsigned_abs()).unwrap_or(count);
        Ok((end, self.pulses_for(end == CentreEnd::Fault, pulses)))
    }

    /// The ticks of the move to the centre, until it ends. A switch the cart
    /// is `leaving` is checked once the cart has moved `release_pulses`: still
    /// pressed, the cart is driving into it.
    fn until_centred(
        &mut self,
        ramp: &mut Ramp,
        target: StopPoints,
        mut leaving: Option<RailEnd>,
        base: i64,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<CentreEnd> {
        let start = self.position;
        loop {
            match sleep::until(|| self.look_centring(leaving, udp, wake).transpose())? {
                CentreWake::Ended(end) => return Ok(end),
                CentreWake::Stop => ramp.command(0),
                CentreWake::Tick => {
                    self.position = start + (self.stepper.position() - base);
                    if let Some(end) = leaving
                        && (self.position - start).unsigned_abs() >= u64::from(RELEASE_PULSES)
                    {
                        if self.pressed(end) {
                            return Ok(CentreEnd::Tripped(end));
                        }
                        // Its bounces as the cart left it don't count.
                        let _ = self.tripped(end);
                        leaving = None;
                    }
                    self.ramp_tick(ramp, target)?;
                    // At rest after a `Stop`. Reaching the centre is the
                    // count stop's to say: braking onto it, the ramp may rest
                    // a pulse short, and starts again at the next tick.
                    if ramp.rate() == 0 && ramp.command_rate() == 0 {
                        return Ok(CentreEnd::Stopped);
                    }
                }
            }
        }
    }

    /// One look during the move to the centre, without sleeping: what has
    /// happened, or None if nothing the move acts on. Only `Stop` and a
    /// command with a bad sign byte, a fault, are taken: any other datagram is
    /// dropped unanswered (real-open-issues.md, issue 5).
    fn look_centring(
        &mut self,
        leaving: Option<RailEnd>,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<Option<CentreWake>> {
        if self.stepper.stopped_at_count() {
            return Ok(Some(CentreWake::Ended(CentreEnd::Centred)));
        }
        for end in [RailEnd::Idler, RailEnd::Motor] {
            if leaving != Some(end) && self.tripped(end) {
                return Ok(Some(CentreWake::Ended(CentreEnd::Tripped(end))));
            }
        }
        // The tick before the datagrams: the rate for the next tick must be
        // queued at least 40 µs before it (unor4-hal `step/step_generator.rs`,
        // `queue`).
        if self.stepper.poll_tick().is_some() {
            return Ok(Some(CentreWake::Tick));
        }
        while let Some(datagram) = udp.look(wake).map_err(MoveError::WakeTimer)? {
            match WifiMessage::parse(&datagram) {
                Some(WifiMessage::Stop) => return Ok(Some(CentreWake::Stop)),
                Some(WifiMessage::BadSign) => {
                    return Ok(Some(CentreWake::Ended(CentreEnd::Fault)));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    /// A move's answer gives the pulses it sent, but the FAULT STATE reply
    /// gives where the cart stopped, counted from the idler's end, as every
    /// reply in FAULT does.
    fn pulses_for(&self, fault: bool, sent: u32) -> u32 {
        if fault {
            self.pulses_from_idler()
        } else {
            sent
        }
    }

    /// Runs the Orin's commands, starting with `first`, until the cart is at
    /// rest again, has entered a protected end zone, or a limit switch is
    /// pressed. A switch is looked at first on every wake, before the tick
    /// and the datagrams; pressed, or read pressed, it ends the move at once,
    /// a fault, whatever the count says (the user, 2026-11-07). Every 2 ms
    /// tick: the count is checked against the stop points, then the ramp
    /// takes one step towards the latest command, the soft limit caps it, and the rate is
    /// queued for the next tick. Each command that arrives re-aims the ramp,
    /// and none is dropped (real-open-issues.md, issue 6); every other
    /// datagram is dropped unanswered (issue 5, MOVING). It sleeps between
    /// ticks, woken by the tick or a datagram.
    pub(crate) fn follow(
        &mut self,
        first: i32,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<MovingEnd> {
        // Commands are taken only in CALIBRATED, which needs the rail.
        let Some(stops) = self.rail.stop_points() else {
            return Ok(MovingEnd::Rest);
        };
        let mut ramp = Ramp::new();
        ramp.command(first);

        let base = self.start_ticks();
        let start = self.position;
        let ended = self.until_at_rest(&mut ramp, stops, base, start, udp, wake);
        self.stop_ticks(base, start);
        ended
    }

    /// Starts a move timed by the 2 ms tick: the tick wakes the board, and
    /// one is taken now. Returns the HAL's position at it, the move's base.
    /// The HAL's position counts each tick's pulses in the direction it ran
    /// last, and the seeks run untimed, so it is right only as a difference
    /// from a tick taken now, with the cart at rest.
    fn start_ticks(&mut self) -> i64 {
        self.stepper.set_tick_wake(true);
        sleep::until(|| self.stepper.poll_tick());
        self.stepper.position()
    }

    /// Ends a move timed by the tick, whatever ended it: stops the pulses,
    /// takes the cart's position from the HAL's, by `base` and the cart's
    /// `start`, and turns the tick's wake off. A pulse that was running ends
    /// and is latched by the next tick, so the count is taken there.
    fn stop_ticks(&mut self, base: i64, start: i64) {
        self.stepper.stop();
        sleep::until(|| self.stepper.poll_tick());
        self.position = start + (self.stepper.position() - base);
        self.stepper.set_tick_wake(false);
    }

    /// The ticks of `follow`, until the cart is at rest or in a protected
    /// zone.
    fn until_at_rest(
        &mut self,
        ramp: &mut Ramp,
        stops: StopPoints,
        base: i64,
        start: i64,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<MovingEnd> {
        loop {
            match sleep::until(|| self.look_moving(udp, wake).transpose())? {
                MovingWake::Command(rate) => ramp.command(rate),
                // A fault: stopped in one jump, by `follow`.
                MovingWake::BadSign => return Ok(MovingEnd::BadSign),
                // A fault: stopped in one jump, by `follow`.
                MovingWake::Switch => return Ok(MovingEnd::Switch),
                MovingWake::Tick => {
                    self.position = start + (self.stepper.position() - base);
                    if stops.passed(self.position) {
                        // Into a protected zone: stopped in one jump (issue 5).
                        return Ok(MovingEnd::EndZone);
                    }
                    let (next, sent) = self.ramp_tick(ramp, stops)?;
                    // At rest when the ramp is at 0 and stays there: the
                    // command is 0, or the soft limit holds the cart at a stop
                    // point. A reversal passing through 0 is neither.
                    if ramp.rate() == 0 && (ramp.command_rate() == 0 || sent != next) {
                        return Ok(MovingEnd::Rest);
                    }
                }
            }
        }
    }

    /// One tick's step of a ramped move, the cart's position just taken: the
    /// ramp's next rate, capped by the soft limit towards `stops`, queued for
    /// the next tick. Returns the rate the ramp asked for and the rate sent.
    fn ramp_tick(&mut self, ramp: &mut Ramp, stops: StopPoints) -> MoveResult<(i32, i32)> {
        let next = ramp.next();
        let sent = stops.cap(next, ramp.rate(), self.position);
        match self.stepper.queue(sent) {
            Ok(_) => ramp.sent(sent),
            // Not this tick: the rate running now holds for one more, and the
            // ramp tries again at the next.
            Err(StepError::NotReady | StepError::TooLate) => {}
            Err(error) => return Err(error.into()),
        }
        Ok((next, sent))
    }

    /// One look while the commands run, without sleeping: a tick, or a
    /// command, or None. Any datagram that isn't a command is dropped.
    fn look_moving(
        &mut self,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<Option<MovingWake>> {
        // The switches before anything else: a press is a fault whatever the
        // tick or the datagrams hold. Two register reads each, well inside
        // the 40 µs the tick below needs.
        if self.switch_hit() {
            return Ok(Some(MovingWake::Switch));
        }
        // The tick next: the rate for the next tick must be queued at least
        // 40 µs before it (unor4-hal `step/step_generator.rs`, `queue`).
        if self.stepper.poll_tick().is_some() {
            return Ok(Some(MovingWake::Tick));
        }
        while let Some(datagram) = udp.look(wake).map_err(MoveError::WakeTimer)? {
            match WifiMessage::parse(&datagram) {
                Some(WifiMessage::Command(rate)) => return Ok(Some(MovingWake::Command(rate))),
                Some(WifiMessage::BadSign) => return Ok(Some(MovingWake::BadSign)),
                _ => {}
            }
        }
        Ok(None)
    }

    /// Where the cart is, counted from the idler's end, for an answer.
    pub(crate) fn pulses_from_idler(&self) -> u32 {
        self.rail.pulses_from_idler(self.position)
    }

    /// Runs the cart at `rate`, a seek's, until something ends the move, then
    /// stops it, adds the pulses it sent to `position`, and returns what ended
    /// it and the pulses. It sleeps the whole way, woken by a switch, a
    /// datagram or the move timer. Only `Stop` and a command with a bad sign
    /// byte, a fault, are taken: any other datagram is dropped unanswered
    /// (real-open-issues.md, issue 5; real-loop-dia.md, *The command*).
    ///
    /// `leaving`: a switch the cart starts on, whose presses count only once
    /// the cart has moved `release_pulses` off it. `limit`: the longest the
    /// move may run.
    fn drive(
        &mut self,
        rate: i32,
        leaving: Option<RailEnd>,
        limit: Duration,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<(MoveEnd, u32)> {
        // Only a press during the move counts.
        let _ = self.tripped(RailEnd::Idler);
        let _ = self.tripped(RailEnd::Motor);

        let start = self.stepper.pulses();
        let ended = self
            .stepper
            .run(rate)
            .map_err(MoveError::from)
            .and_then(|_| self.until_move_ends(rate, leaving, limit, udp, wake));
        // Stopped whatever happened, an error included: no pulse is cut, and
        // the last one is counted before `stop` returns (unor4-hal
        // `step/step_generator.rs`, `stop`).
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

    /// Sleeps until the move at `rate` is over, and says what ended it. A
    /// switch the cart is `leaving` is checked once the cart should be off
    /// it: still pressed, the cart is driving into it. The move timer times
    /// that check first, then what is left of `limit`.
    fn until_move_ends(
        &mut self,
        rate: i32,
        mut leaving: Option<RailEnd>,
        limit: Duration,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<MoveEnd> {
        let release = release_time(rate);
        let first = if leaving.is_some() { release } else { limit };
        let mut deadline = self.deadline(first)?;
        loop {
            let woke = sleep::until(|| self.look(leaving, &mut deadline, udp, wake).transpose())?;
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
                    deadline = self.deadline(limit.saturating_sub(release))?;
                }
            }
        }
    }

    /// One look during a move, without sleeping: what has happened, or None
    /// if nothing the move acts on.
    fn look(
        &mut self,
        leaving: Option<RailEnd>,
        deadline: &mut Deadline,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<Option<MoveWake>> {
        for end in [RailEnd::Idler, RailEnd::Motor] {
            if leaving != Some(end) && self.tripped(end) {
                return Ok(Some(MoveWake::Ended(MoveEnd::Switch(end))));
            }
        }
        while let Some(datagram) = udp.look(wake).map_err(MoveError::WakeTimer)? {
            match WifiMessage::parse(&datagram) {
                Some(WifiMessage::Stop) => return Ok(Some(MoveWake::Ended(MoveEnd::Stopped))),
                Some(WifiMessage::BadSign) => return Ok(Some(MoveWake::Ended(MoveEnd::BadSign))),
                _ => {}
            }
        }
        if deadline
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

    /// Starts `wait` on the move timer.
    fn deadline(&mut self, wait: Duration) -> MoveResult<Deadline> {
        Deadline::start(&mut self.timer, wait).map_err(MoveError::MoveTimer)
    }

    /// Whether either switch was pressed since it was last looked at, even if
    /// released again, or reads pressed now. In CALIBRATED and MOVING either
    /// is a fault (the user, 2026-11-07). Each switch's press is latched in
    /// hardware and wakes the board (`pins/*_limit_switch.rs`), so a press
    /// is never missed between looks; reading the level as well catches a
    /// switch already held, or a broken wire, which reads pressed.
    pub(crate) fn switch_hit(&mut self) -> bool {
        let idler = self.tripped(RailEnd::Idler) | self.pressed(RailEnd::Idler);
        let motor = self.tripped(RailEnd::Motor) | self.pressed(RailEnd::Motor);
        idler | motor
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
/// comes early. A seek runs at one rate, start to stop.
fn release_time(rate: i32) -> Duration {
    let speed = u64::from(rate.unsigned_abs()).max(1);
    Duration::from_micros((u64::from(RELEASE_PULSES) * MICROS_PER_SECOND).div_ceil(speed))
}
