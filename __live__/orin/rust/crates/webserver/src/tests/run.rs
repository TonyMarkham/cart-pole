//! `/run/status`'s answer: the brain's status with the board's `state` answer
//! added, as `/run` reads it (`real-open-issues.md`, issue 7, step 2).

use crate::run::with_board;

use serde_json::{Value, json};

fn idle() -> Value {
    json!({"state": "idle", "message": "stopped", "run_dir": null, "frames": 0})
}

#[test]
fn given_the_boards_answer_when_added_then_it_is_passed_on_whole() {
    let answer = r#"{"status":200,"result":"calibrated","message":"the rail is known and the cart is at rest","pin":null,"pulses":0}"#;
    let status = with_board(idle(), Some(Ok(answer.to_string())));
    assert_eq!(status["board"]["result"], "calibrated");
    assert_eq!(status["board"]["status"], 200);
    assert_eq!(status["message"], "stopped");
    assert!(status.get("board_error").is_none());
}

#[test]
fn given_no_answer_from_the_board_when_added_then_board_is_null_with_the_reason() {
    let status = with_board(idle(), Some(Err("The board did not answer.".to_string())));
    assert_eq!(status["board"], Value::Null);
    assert_eq!(status["board_error"], "The board did not answer.");
}

#[test]
fn given_a_run_when_the_board_was_not_asked_then_board_is_null_without_a_reason() {
    let running = json!({"state": "run", "message": "", "run_dir": "x", "frames": 12});
    let status = with_board(running, None);
    assert_eq!(status["board"], Value::Null);
    assert!(status.get("board_error").is_none());
    assert_eq!(status["frames"], 12);
}

#[test]
fn given_an_answer_that_is_not_json_when_added_then_it_is_named_in_board_error() {
    let status = with_board(idle(), Some(Ok("pins".to_string())));
    assert_eq!(status["board"], Value::Null);
    assert_eq!(
        status["board_error"],
        "The board's answer was not its JSON: pins"
    );
}
