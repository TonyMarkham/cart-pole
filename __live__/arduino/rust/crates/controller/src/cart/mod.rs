pub(crate) mod error;
pub(crate) mod move_timer;
pub(crate) mod rail_end;
pub(crate) mod seek_end;
pub(crate) mod seek_wake;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    cart::{
        error::{move_error::MoveError, move_result::MoveResult},
        rail_end::RailEnd,
        seek_end::SeekEnd,
        seek_wake::SeekWake,
    },
    config::calibration::{SEEK_RELEASE_US, SEEK_SPEED, SEEK_TIMEOUT_MS},
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
/// limit switch at each end, and the move timer, which times a move's own
/// limits.
pub(crate) struct Cart {
    stepper: StepGenerator,
    idler: Switch<D3, Nc>,
    motor: Switch<D2, Nc>,
    timer: WakeTimer,
}

impl Cart {
    /// Takes the parts, and turns the motor driver on: from here on it holds
    /// the cart in place between moves. Its ENA lead (5 µs) has long passed
    /// by the first move, which can only come after the WiFi join.
    pub(crate) fn new(
        mut stepper: StepGenerator,
        idler: Switch<D3, Nc>,
        motor: Switch<D2, Nc>,
        timer: WakeTimer,
    ) -> Self {
        stepper.enable();
        Self {
            stepper,
            idler,
            motor,
            timer,
        }
    }

    /// Seeks the end `toward`: runs at `seek_speed` until that end's switch
    /// trips, then stops. Returns how the seek ended and the pulses it sent.
    /// It sleeps the whole way, woken by a switch, a datagram or the move
    /// timer. During the seek only `Stop` is taken: any other datagram is
    /// dropped unanswered (real-open-issues.md, issue 5).
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

        // Only a press during the seek counts.
        let _ = self.tripped(RailEnd::Idler);
        let _ = self.tripped(RailEnd::Motor);

        let start = self.stepper.pulses();
        self.stepper.run(toward.rate(SEEK_SPEED))?;
        let ended = self.until_seek_ends(toward, udp, wake, on_other);
        // Stopped whatever happened, an error included: no pulse is cut, and
        // the last one is counted before `stop` returns (unor4-hal
        // `step/step_generator.rs`, `stop`).
        self.stepper.stop();
        self.timer.cancel();
        let pulses = self.stepper.pulses().wrapping_sub(start);
        Ok((ended?, pulses))
    }

    /// Sleeps until the seek `toward` an end is over, and says how it ended.
    /// `on_other`: the cart started on the other end's switch, so that
    /// switch's presses are ignored until the cart has moved `release_pulses`
    /// off it, and it must read released then.
    fn until_seek_ends(
        &mut self,
        toward: RailEnd,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
        on_other: bool,
    ) -> MoveResult<SeekEnd> {
        let release = Duration::from_micros(SEEK_RELEASE_US);
        let timeout = Duration::from_millis(u64::from(SEEK_TIMEOUT_MS));

        let mut leaving_other = on_other;
        let first = if leaving_other { release } else { timeout };
        let mut limit = Deadline::start(&mut self.timer, first).map_err(MoveError::MoveTimer)?;
        loop {
            let woke = sleep::until(|| {
                self.look(toward, udp, wake, leaving_other, &mut limit)
                    .transpose()
            })?;
            match woke {
                SeekWake::Ended(end) => return Ok(end),
                SeekWake::LeftSwitch => {
                    // Still pressed: the cart is driving into it.
                    if self.pressed(toward.other()) {
                        return Ok(SeekEnd::OtherEnd);
                    }
                    // Its bounces as the cart left it don't count.
                    let _ = self.tripped(toward.other());
                    leaving_other = false;
                    limit = Deadline::start(&mut self.timer, timeout.saturating_sub(release))
                        .map_err(MoveError::MoveTimer)?;
                }
            }
        }
    }

    /// One look during a seek `toward` an end, without sleeping: what has
    /// happened, or None if nothing the seek acts on.
    fn look(
        &mut self,
        toward: RailEnd,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
        leaving_other: bool,
        limit: &mut Deadline,
    ) -> MoveResult<Option<SeekWake>> {
        if self.tripped(toward) {
            return Ok(Some(SeekWake::Ended(SeekEnd::Arrived)));
        }
        if !leaving_other && self.tripped(toward.other()) {
            return Ok(Some(SeekWake::Ended(SeekEnd::OtherEnd)));
        }
        while let Some(datagram) = udp.look(wake).map_err(MoveError::WakeTimer)? {
            if WifiMessage::parse(&datagram) == Some(WifiMessage::Stop) {
                return Ok(Some(SeekWake::Ended(SeekEnd::Stopped)));
            }
        }
        if limit
            .passed(&mut self.timer)
            .map_err(MoveError::MoveTimer)?
        {
            return Ok(Some(if leaving_other {
                SeekWake::LeftSwitch
            } else {
                SeekWake::Ended(SeekEnd::TimedOut)
            }));
        }
        Ok(None)
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
