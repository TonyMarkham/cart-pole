//! Run's rule against `/run/status` answers in the webserver's shape: Run only
//! when the brain is idle and the board answers `"calibrated"`, and otherwise
//! issue 7's reason.

use crate::{look::Look, run_rule::why_not};

fn idle_with(board: &str) -> Look {
    let text = format!(
        r#"{{"state":"idle","message":"stopped","run_dir":null,"frames":0,"board":{board}}}"#
    );
    Look::Status(serde_json::from_str(&text).unwrap())
}

#[test]
fn given_an_idle_brain_and_a_calibrated_board_when_ruled_then_run_is_allowed() {
    let look = idle_with(
        r#"{"status":200,"result":"calibrated","message":"the rail is known","pin":null,"pulses":0}"#,
    );
    assert_eq!(why_not(&look), None);
}

#[test]
fn given_an_uncalibrated_board_when_ruled_then_the_reason_says_calibrate() {
    let look = idle_with(r#"{"status":200,"result":"uncalibrated","message":""}"#);
    assert_eq!(
        why_not(&look).as_deref(),
        Some("The board is not calibrated: run the calibration first.")
    );
}

#[test]
fn given_the_fault_state_reply_when_ruled_then_the_reason_says_reset() {
    let look = idle_with(
        r#"{"status":409,"result":"fault","message":"the board is in FAULT: reset it, then calibrate","pin":null,"pulses":58839}"#,
    );
    assert_eq!(
        why_not(&look).as_deref(),
        Some("The board is in FAULT: reset the board, then calibrate.")
    );
}

#[test]
fn given_no_board_answer_when_ruled_then_the_webservers_reason_is_shown() {
    let text = r#"{"state":"idle","message":"","run_dir":null,"frames":0,"board":null,"board_error":"The board did not answer: no answer within 1000 ms. It is off, or the message was lost."}"#;
    let look = Look::Status(serde_json::from_str(text).unwrap());
    assert_eq!(
        why_not(&look).as_deref(),
        Some(
            "The board did not answer: no answer within 1000 ms. It is off, or the message was lost."
        )
    );
}

#[test]
fn given_a_run_when_ruled_then_only_stop_is_left() {
    let text =
        r#"{"state":"run","message":"","run_dir":"runs/20261109_120000","frames":40,"board":null}"#;
    let look = Look::Status(serde_json::from_str(text).unwrap());
    assert_eq!(why_not(&look).as_deref(), Some("A run is going."));
}

#[test]
fn given_no_brain_when_ruled_then_the_reason_says_so() {
    let look = Look::NoBrain("the brain did not answer, so it may not be running".to_string());
    assert_eq!(why_not(&look).as_deref(), Some("The brain did not answer."));
}
