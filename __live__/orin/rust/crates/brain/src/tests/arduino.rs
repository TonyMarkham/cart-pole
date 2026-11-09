use crate::arduino::board_reply::BoardReply;

#[test]
fn given_the_fault_state_reply_when_parsed_then_it_ends_the_run_naming_the_pulses() {
    // Given
    let datagram = br#"{"status":409,"result":"fault","message":"","pin":null,"pulses":39059}"#;

    // When
    let reply = BoardReply::parse(datagram).unwrap();

    // Then
    assert_eq!(
        reply.ends_run().unwrap(),
        "the board is in FAULT, at 39059 pulses: reset it, then calibrate"
    );
}

#[test]
fn given_the_uncalibrated_reply_when_parsed_then_it_ends_the_run() {
    // Given
    let datagram = br#"{"status":409,"result":"uncalibrated","message":"","pin":null,"pulses":0}"#;

    // When
    let reply = BoardReply::parse(datagram).unwrap();

    // Then
    assert_eq!(
        reply.ends_run().unwrap(),
        "the board is not calibrated: calibrate first"
    );
}

#[test]
fn given_a_state_answer_when_parsed_then_it_ends_nothing() {
    // Given
    let datagram =
        br#"{"status":200,"result":"calibrated","message":"","pin":null,"pulses":29914}"#;

    // When
    let reply = BoardReply::parse(datagram).unwrap();

    // Then
    assert_eq!(reply.ends_run(), None);
}

#[test]
fn given_bytes_that_are_not_a_reply_when_parsed_then_there_is_none() {
    // Given
    let datagram = b"\x01\x10\x27";

    // When
    let reply = BoardReply::parse(datagram);

    // Then
    assert!(reply.is_none());
}
