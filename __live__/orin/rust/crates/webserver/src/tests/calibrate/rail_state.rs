use crate::calibrate::{rail_state::RailState, stop::Stop};

/// A move's answer as the board sends it, with only the keys `note` reads.
fn answered(status: u16, result: &str, pulses: u32) -> Vec<u8> {
    format!("{{\"status\":{status},\"result\":\"{result}\",\"pulses\":{pulses}}}").into_bytes()
}

/// A rail whose span is known: a trip at the motor's end, then at the idler's.
fn spanned() -> RailState {
    let mut rail = RailState::default();
    rail.note(Stop::Motor, &answered(200, "tripped", 1));
    rail.note(Stop::Idler, &answered(200, "tripped", 59_822));
    rail
}

#[test]
fn given_no_trip_yet_when_one_end_trips_then_the_span_is_unknown() {
    // Given
    let mut rail = RailState::default();

    // When
    rail.note(Stop::Motor, &answered(200, "tripped", 29_920));

    // Then
    assert_eq!(rail.span, None);
    assert_eq!(rail.last_stop, Some(Stop::Motor));
}

#[test]
fn given_a_trip_at_one_end_when_the_other_end_trips_then_its_pulses_are_the_span() {
    // Given
    let mut rail = RailState::default();
    rail.note(Stop::Motor, &answered(200, "tripped", 29_920));

    // When
    rail.note(Stop::Idler, &answered(200, "tripped", 59_822));

    // Then
    assert_eq!(rail.span, Some(59_822));
    assert_eq!(rail.last_stop, Some(Stop::Idler));
}

#[test]
fn given_a_known_span_when_a_move_is_stopped_then_the_span_stays_but_the_place_is_lost() {
    // Given
    let mut rail = spanned();

    // When
    rail.note(Stop::Motor, &answered(409, "stopped", 100));

    // Then
    assert_eq!(rail.span, Some(59_822));
    assert_eq!(rail.last_stop, None);
}

#[test]
fn given_a_stopped_move_when_the_next_end_trips_then_it_measures_no_new_span() {
    // Given
    let mut rail = spanned();
    rail.note(Stop::Motor, &answered(409, "stopped", 100));

    // When
    rail.note(Stop::Idler, &answered(200, "tripped", 100));

    // Then
    assert_eq!(rail.span, Some(59_822));
}

#[test]
fn given_a_known_span_when_the_cart_is_centred_then_the_span_stays() {
    // Given
    let mut rail = spanned();

    // When
    rail.note(Stop::Centre, &answered(200, "centred", 29_911));

    // Then
    assert_eq!(rail.span, Some(59_822));
    assert_eq!(rail.last_stop, Some(Stop::Centre));
}

#[test]
fn given_a_known_span_when_a_move_times_out_then_everything_is_forgotten() {
    // Given
    let mut rail = spanned();

    // When
    rail.note(Stop::Motor, &answered(408, "timed_out", 9));

    // Then
    assert_eq!(rail, RailState::default());
}

#[test]
fn given_a_known_span_when_the_wrong_switch_trips_then_everything_is_forgotten() {
    // Given
    let mut rail = spanned();

    // When
    rail.note(Stop::Motor, &answered(409, "tripped", 9));

    // Then
    assert_eq!(rail, RailState::default());
}

#[test]
fn given_a_trip_when_the_answer_says_nothing_moved_then_nothing_changes() {
    // Given
    let mut rail = RailState::default();
    rail.note(Stop::Motor, &answered(200, "tripped", 1));
    let before = rail;

    // When
    rail.note(Stop::Motor, &answered(409, "already_there", 0));

    // Then
    assert_eq!(rail, before);
}
