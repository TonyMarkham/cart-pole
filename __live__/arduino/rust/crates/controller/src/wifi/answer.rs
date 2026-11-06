use crate::{
    constants::{STATUS_CONFLICT, STATUS_OK},
    state::State,
};

use core::fmt;

/// One answer to the Orin, in the board's one format, five keys: `status`,
/// `result`, `message`, `pin` and `pulses`.
/// Nothing yet moves the cart or trips a switch, so `pin` is always null and
/// `pulses` always 0.
pub(crate) struct Answer {
    status: u16,
    result: &'static str,
    message: &'static str,
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
            }),
            _ => None,
        }
    }

    /// The answer as JSON. Every string here is the firmware's own, with no
    /// quote or backslash in it, so none needs escaping.
    pub(crate) fn write(&self, out: &mut impl fmt::Write) -> fmt::Result {
        write!(
            out,
            "{{\"status\":{},\"result\":\"{}\",\"message\":\"{}\",\"pin\":null,\"pulses\":0}}",
            self.status, self.result, self.message
        )
    }
}
