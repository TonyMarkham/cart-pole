//! STOP's words, from the webserver's answers to `/run/stop`
//! (`__live__/orin/rust/crates/webserver/src/run/mod.rs`, `stop`).

use crate::stop_answer::StopAnswer;

#[test]
fn given_an_idle_brain_and_zeros_sent_when_worded_then_both_are_said() {
    let answer: StopAnswer = serde_json::from_str(
        r#"{"state":"idle","message":"stopped","run_dir":"x","frames":9,"zeros_sent":true}"#,
    )
    .unwrap();
    assert_eq!(
        answer.words(),
        "STOP: The board was sent the command 0. The brain is idle."
    );
}

#[test]
fn given_no_brain_when_worded_then_the_zeros_still_went_out() {
    let answer: StopAnswer = serde_json::from_str(
        r#"{"state":"unknown","message":"the brain did not answer","zeros_sent":true}"#,
    )
    .unwrap();
    assert_eq!(
        answer.words(),
        "STOP: The board was sent the command 0. The brain did not say it is idle: the brain did not answer"
    );
}

#[test]
fn given_the_zeros_failed_when_worded_then_the_e_stop_is_named() {
    let answer: StopAnswer = serde_json::from_str(
        r#"{"state":"idle","message":"","zeros_sent":false,"zeros_error":"network unreachable"}"#,
    )
    .unwrap();
    assert!(answer.words().starts_with(
        "STOP: The command 0 could not be sent: network unreachable. Use the e-stop."
    ));
}
