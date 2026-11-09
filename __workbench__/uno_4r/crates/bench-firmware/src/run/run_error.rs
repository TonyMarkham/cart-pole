use core::fmt;
use unor4_hal::{step::StepError, wake::WakeError};

/// Why the run loop stopped, for `OK STATUS stopped run: …`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum RunError {
    /// The wake timer refused a wait (a datagram's frame deadline).
    Wake(WakeError),
    /// The step generator refused a rate.
    Step(StepError),
}

impl From<WakeError> for RunError {
    fn from(error: WakeError) -> Self {
        Self::Wake(error)
    }
}

impl From<StepError> for RunError {
    fn from(error: StepError) -> Self {
        Self::Step(error)
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Wake(error) => write!(f, "wake timer: {error:?}"),
            Self::Step(error) => write!(f, "step generator: {error:?}"),
        }
    }
}
