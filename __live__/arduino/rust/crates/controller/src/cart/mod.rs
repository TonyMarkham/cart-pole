pub(crate) mod error;
pub(crate) mod move_timer;
pub(crate) mod seek_end;
pub(crate) mod seek_wake;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    cart::{error::move_result::MoveResult, seek_end::SeekEnd, seek_wake::SeekWake},
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

    /// Seeks the idler's end: runs at `seek_speed` until the idler's switch
    /// trips, then stops. Returns how the seek ended and the pulses it sent.
    /// It sleeps the whole way, woken by a switch, a datagram or the move
    /// timer. During the seek only `Stop` is taken: any other datagram is
    /// dropped unanswered (real-open-issues.md, issue 5).
    pub(crate) fn seek_idler(
        &mut self,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
    ) -> MoveResult<(SeekEnd, u32)> {
        let at_idler = self.idler.is_active();
        let on_motor = self.motor.is_active();
        if at_idler && on_motor {
            return Ok((SeekEnd::Refused, 0));
        }
        if at_idler {
            return Ok((SeekEnd::AlreadyThere, 0));
        }

        // Only a press during the seek counts.
        let _ = self.idler.activated();
        let _ = self.motor.activated();

        let start = self.stepper.pulses();
        // A positive rate runs towards the motor's end (real-open-issues.md,
        // issue 4), so the idler's end is negative.
        self.stepper.run(-SEEK_SPEED)?;
        let ended = self.until_seek_ends(udp, wake, on_motor);
        // Stopped whatever happened, an error included: no pulse is cut, and
        // the last one is counted before `stop` returns (unor4-hal
        // `step/step_generator.rs`, `stop`).
        self.stepper.stop();
        self.timer.cancel();
        let pulses = self.stepper.pulses().wrapping_sub(start);
        Ok((ended?, pulses))
    }

    /// Sleeps until the seek is over, and says how it ended. `on_motor`: the
    /// cart started on the motor's switch, so that switch's presses are
    /// ignored until the cart has moved `release_pulses` off it, and it must
    /// read released then.
    fn until_seek_ends(
        &mut self,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
        on_motor: bool,
    ) -> MoveResult<SeekEnd> {
        let release = Duration::from_micros(SEEK_RELEASE_US);
        let timeout = Duration::from_millis(u64::from(SEEK_TIMEOUT_MS));

        let mut leaving_motor = on_motor;
        let first = if leaving_motor { release } else { timeout };
        let mut limit = Deadline::start(&mut self.timer, first)?;
        loop {
            let woke =
                sleep::until(|| self.look(udp, wake, leaving_motor, &mut limit).transpose())?;
            match woke {
                SeekWake::Ended(end) => return Ok(end),
                SeekWake::LeftSwitch => {
                    // Still pressed: the cart is driving into it.
                    if self.motor.is_active() {
                        return Ok(SeekEnd::Motor);
                    }
                    // Its bounces as the cart left it don't count.
                    let _ = self.motor.activated();
                    leaving_motor = false;
                    limit = Deadline::start(&mut self.timer, timeout.saturating_sub(release))?;
                }
            }
        }
    }

    /// One look during a seek, without sleeping: what has happened, or None
    /// if nothing the seek acts on.
    fn look(
        &mut self,
        udp: &mut UdpSocket,
        wake: &mut WakeTimer,
        leaving_motor: bool,
        limit: &mut Deadline,
    ) -> MoveResult<Option<SeekWake>> {
        if self.idler.activated() {
            return Ok(Some(SeekWake::Ended(SeekEnd::Idler)));
        }
        if !leaving_motor && self.motor.activated() {
            return Ok(Some(SeekWake::Ended(SeekEnd::Motor)));
        }
        while let Some(datagram) = udp.look(wake)? {
            if WifiMessage::parse(&datagram) == Some(WifiMessage::Stop) {
                return Ok(Some(SeekWake::Ended(SeekEnd::Stopped)));
            }
        }
        if limit.passed(&mut self.timer)? {
            return Ok(Some(if leaving_motor {
                SeekWake::LeftSwitch
            } else {
                SeekWake::Ended(SeekEnd::TimedOut)
            }));
        }
        Ok(None)
    }
}
