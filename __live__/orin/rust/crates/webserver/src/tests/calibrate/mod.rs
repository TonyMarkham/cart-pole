mod rail_state;

// ---------------------------------------------------------------------------------------------- //

use crate::calibrate::move_status;

use axum::http::StatusCode;

#[test]
fn given_a_pins_answer_when_read_as_a_move_then_it_is_not_one() {
    // Given
    let pins = b"{\"d2\":0,\"d3\":0,\"d4\":1,\"d5\":1,\"d6\":1}";

    // When
    let status = move_status(pins);

    // Then
    assert_eq!(status, None);
}

#[test]
fn given_text_that_is_not_json_when_read_as_a_move_then_it_is_not_one() {
    // Given
    let nonsense = b"not json";

    // When
    let status = move_status(nonsense);

    // Then
    assert_eq!(status, None);
}

#[test]
fn given_a_status_no_http_code_has_when_read_as_a_move_then_it_is_not_one() {
    // Given
    let out_of_range = b"{\"status\":70000}";

    // When
    let status = move_status(out_of_range);

    // Then
    assert_eq!(status, None);
}

#[test]
fn given_a_moves_answer_when_read_then_its_status_is_the_http_status() {
    // Given
    let stopped = br#"{"status":409,"result":"stopped","message":"m","pin":null,"pulses":1}"#;

    // When
    let status = move_status(stopped);

    // Then
    assert_eq!(status, Some(StatusCode::CONFLICT));
}
