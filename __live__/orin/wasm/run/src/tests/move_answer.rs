//! Re-centre's words, from the controller firmware's answers to
//! `MoveToCentre` (`Answer::centre`) and the webserver's own refusals.

use crate::move_answer::MoveAnswer;

#[test]
fn given_a_centred_answer_when_worded_then_it_says_done() {
    let answer: MoveAnswer = serde_json::from_str(
        r#"{"status":200,"result":"centred","message":"the cart stopped at the centre","pin":null,"pulses":29910}"#,
    )
    .unwrap();
    assert_eq!(
        answer.words(),
        "Re-centre done: the cart stopped at the centre."
    );
}

#[test]
fn given_the_webservers_refusal_during_a_run_when_worded_then_its_reason_is_shown() {
    let answer: MoveAnswer = serde_json::from_str(
        r#"{"status":409,"result":"busy","message":"a run is going, and the brain alone moves the cart: STOP it first","pin":null,"pulses":0}"#,
    )
    .unwrap();
    assert_eq!(
        answer.words(),
        "Re-centre did not arrive (busy): a run is going, and the brain alone moves the cart: STOP it first."
    );
}
