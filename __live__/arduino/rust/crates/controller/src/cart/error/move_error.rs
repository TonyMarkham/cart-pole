use unor4_hal::{step::StepError, wake::WakeError};

/// Why a move couldn't run: the step generator refused it, or a timer
/// couldn't be started. Either is the firmware's own fault, not the rig's,
/// so the board stops for good and says which (`OK STATUS stopped …`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveError {
    Step(StepError),
    Wake(WakeError),
}

impl From<StepError> for MoveError {
    fn from(error: StepError) -> Self {
        Self::Step(error)
    }
}

impl From<WakeError> for MoveError {
    fn from(error: WakeError) -> Self {
        Self::Wake(error)
    }
}
