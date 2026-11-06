use core::fmt;
use unor4_hal::{step::StepError, wake::WakeError};

/// Why a move couldn't run, in words for `OK STATUS stopped seek: …`. Each is
/// the firmware's own fault, not the rig's: the rate is fixed at build time,
/// the driver is on from start-up, and every wait is at most 1.398 s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveError {
    /// The step generator refused the move's rate.
    StepGenerator(StepError),
    /// The move timer refused a wait: the move's own time limit.
    MoveTimer(WakeError),
    /// The wake timer refused a wait: a datagram's frame, read during the
    /// move.
    WakeTimer(WakeError),
}

impl From<StepError> for MoveError {
    fn from(error: StepError) -> Self {
        Self::StepGenerator(error)
    }
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::StepGenerator(error) => write!(f, "step generator: {error:?}"),
            Self::MoveTimer(error) => write!(f, "move timer: {error:?}"),
            Self::WakeTimer(error) => write!(f, "wake timer: {error:?}"),
        }
    }
}
