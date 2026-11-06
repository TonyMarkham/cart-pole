use crate::{
    cart::{centre_end::CentreEnd, rail_end::RailEnd, seek_end::SeekEnd},
    constants::{STATUS_BAD_REQUEST, STATUS_CONFLICT, STATUS_OK, STATUS_TIMEOUT},
    state::State,
};

use core::fmt;

/// One answer to the Orin, in the board's one format, five keys: `status`,
/// `result`, `message`, `pin` and `pulses`. `pin` is the switch a move
/// stopped at, or null; `pulses` is how far the move went, or 0.
pub(crate) struct Answer {
    status: u16,
    result: &'static str,
    message: &'static str,
    pin: Option<&'static str>,
    pulses: u32,
}

impl Answer {
    /// The answer to `state`: 200, and the state's name (issue 7, step 1).
    pub(crate) const fn state(state: State) -> Self {
        Self {
            status: STATUS_OK,
            result: state.name(),
            message: match state {
                State::Uncalibrated => {
                    "the board knows nothing of the rail: run the calibration first"
                }
                State::Calibrating => "a calibration is under way",
                State::Calibrated => "the rail is known and the cart is at rest",
                State::Moving => "the cart is moving",
                State::Fault => {
                    "the cart entered a protected end zone: reset the board, then calibrate"
                }
            },
            pin: None,
            pulses: 0,
        }
    }

    /// The answer to a command the board doesn't apply in `state`, or None
    /// where the board applies it. Only UNCALIBRATED is answered yet: 409,
    /// `"uncalibrated"` (issue 5, step 2).
    pub(crate) const fn command_refused(state: State) -> Option<Self> {
        match state {
            State::Uncalibrated => Some(Self {
                status: STATUS_CONFLICT,
                result: "uncalibrated",
                message: "no command is applied before a calibration",
                pin: None,
                pulses: 0,
            }),
            _ => None,
        }
    }

    /// The answer to a seek `toward` an end that ended at `end`, after
    /// `pulses`. The results and statuses are the ones the webserver's
    /// calibrate page reads (`__live__/orin/rust/crates/webserver/src/rail_state.rs`):
    /// only a trip of the switch the seek was for is a 200, and `pin` names
    /// the switch.
    pub(crate) const fn seek(toward: RailEnd, end: SeekEnd, pulses: u32) -> Self {
        let (status, result, message, pin) = match end {
            SeekEnd::Arrived => (
                STATUS_OK,
                "tripped",
                "the cart stopped at that end's switch",
                Some(toward.pin()),
            ),
            SeekEnd::OtherEnd => (
                STATUS_CONFLICT,
                "tripped",
                "the cart stopped at the other end's switch",
                Some(toward.other().pin()),
            ),
            SeekEnd::AlreadyThere => (
                STATUS_CONFLICT,
                "already_there",
                "that end's switch already reads pressed: the cart did not move",
                Some(toward.pin()),
            ),
            SeekEnd::Refused => (
                STATUS_CONFLICT,
                "refused",
                "both switches read pressed: the cart did not move",
                None,
            ),
            SeekEnd::Stopped => (
                STATUS_CONFLICT,
                "stopped",
                "a Stop arrived: the cart stopped",
                None,
            ),
            SeekEnd::TimedOut => (
                STATUS_TIMEOUT,
                "timed_out",
                "no switch tripped in time: the cart stopped",
                None,
            ),
        };
        Self {
            status,
            result,
            message,
            pin,
            pulses,
        }
    }

    /// The answer to a move to the centre that ended at `end`, after
    /// `pulses`. The results and statuses are the ones the webserver's
    /// calibrate page reads (`__live__/orin/rust/crates/webserver/src/rail_state.rs`):
    /// only `centred` is a 200, and it makes the board CALIBRATED.
    pub(crate) const fn centre(end: CentreEnd, pulses: u32) -> Self {
        let (status, result, message, pin) = match end {
            CentreEnd::Centred => (STATUS_OK, "centred", "the cart stopped at the centre", None),
            CentreEnd::Tripped(end) => (
                STATUS_CONFLICT,
                "tripped",
                "a switch tripped on the way: seek both ends again",
                Some(end.pin()),
            ),
            CentreEnd::Stopped => (
                STATUS_CONFLICT,
                "stopped",
                "a Stop arrived: the cart stopped",
                None,
            ),
            CentreEnd::NotReady => (
                STATUS_CONFLICT,
                "not_ready",
                "the rail is not known yet: seek both ends first",
                None,
            ),
            CentreEnd::BadSpeed => (
                STATUS_BAD_REQUEST,
                "bad_speed",
                "the speed must be a whole number from 1 to centre_speed_max",
                None,
            ),
            CentreEnd::Refused => (
                STATUS_CONFLICT,
                "refused",
                "both switches read pressed: the cart did not move",
                None,
            ),
        };
        Self {
            status,
            result,
            message,
            pin,
            pulses,
        }
    }

    /// The answer as JSON. Every string here is the firmware's own, with no
    /// quote or backslash in it, so none needs escaping.
    pub(crate) fn write(&self, out: &mut impl fmt::Write) -> fmt::Result {
        write!(
            out,
            "{{\"status\":{},\"result\":\"{}\",\"message\":\"{}\",\"pin\":",
            self.status, self.result, self.message
        )?;
        match self.pin {
            Some(pin) => write!(out, "\"{pin}\"")?,
            None => out.write_str("null")?,
        }
        write!(out, ",\"pulses\":{}}}", self.pulses)
    }
}
