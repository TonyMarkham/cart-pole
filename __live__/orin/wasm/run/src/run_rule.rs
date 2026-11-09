//! Run's rule: Run is pressable only while the brain is idle and the board
//! answers `"calibrated"`; otherwise the page shows why beside it
//! (`real-loop-dia.md`, *The run and idle states*; `real-open-issues.md`,
//! issue 7, step 2, whose five reasons these are, word for word where it
//! gives them).

use crate::{
    constants::{
        BOARD_CALIBRATED, BOARD_CALIBRATING, BOARD_FAULT, BOARD_UNCALIBRATED, BRAIN_IDLE, BRAIN_RUN,
    },
    look::Look,
};

/// None when Run may be pressed, otherwise the reason it may not.
pub(crate) fn why_not(look: &Look) -> Option<String> {
    let status = match look {
        Look::Status(status) => status,
        Look::NoBrain(_) => return Some("The brain did not answer.".to_string()),
        Look::NoWebserver(_) => return Some("The Orin's webserver did not answer.".to_string()),
    };
    match status.state.as_str() {
        BRAIN_RUN => return Some("A run is going.".to_string()),
        BRAIN_IDLE => {}
        other => return Some(format!("The brain says it is \"{other}\", not idle.")),
    }
    let Some(board) = &status.board else {
        return Some(
            status
                .board_error
                .clone()
                .unwrap_or_else(|| "The board did not answer.".to_string()),
        );
    };
    match board.result.as_str() {
        BOARD_CALIBRATED => None,
        BOARD_UNCALIBRATED => {
            Some("The board is not calibrated: run the calibration first.".to_string())
        }
        BOARD_FAULT => Some("The board is in FAULT: reset the board, then calibrate.".to_string()),
        // Between the calibration's moves (issue 7, step 1, *Follows*).
        BOARD_CALIBRATING => {
            Some("The board is calibrating: finish the calibration first.".to_string())
        }
        other => Some(format!("The board says \"{other}\": {}", board.message)),
    }
}
