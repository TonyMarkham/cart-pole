use crate::constants::MOVE_CENTRED;

use serde::Deserialize;

/// The part of a move's answer this page uses, in the board's five-key format
/// or the webserver's copy of it when the board gave none: how the move ended
/// and the words for it. `result` `"centred"` is a move to the centre that
/// arrived (the controller firmware's `Answer::centre`).
#[derive(Debug, Deserialize)]
pub(crate) struct MoveAnswer {
    pub(crate) result: String,
    #[serde(default)]
    pub(crate) message: String,
}

impl MoveAnswer {
    /// What Re-centre did, in words for the page. A board that took the move
    /// and did not arrive stays CALIBRATING, which the reason beside Run then
    /// says.
    pub(crate) fn words(&self) -> String {
        if self.result == MOVE_CENTRED {
            format!("Re-centre done: {}.", self.message)
        } else {
            format!(
                "Re-centre did not arrive ({}): {}.",
                self.result, self.message
            )
        }
    }
}
