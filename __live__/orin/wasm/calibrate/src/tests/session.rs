use crate::{
    board::{answer::Answer, reading::Reading, switches::Switches, travel::Travel},
    session::{Session, heading::Heading, place::Place},
    tone::Tone,
};

/// A move's answer from the board, with only what the session reads.
fn answer(status: u16, result: &str, pulses: u32) -> Answer {
    Answer {
        status,
        result: result.to_string(),
        message: "said".to_string(),
        pin: None,
        pulses,
    }
}

/// One whole move on the page: it starts, and the board answers.
fn run(session: &mut Session, travel: Travel, status: u16, result: &str, pulses: u32) {
    assert!(session.begin(travel, 1_000.0));
    session.finish(answer(status, result, pulses), 2_000.0, "now".to_string());
}

/// A reading of the two switches, 1 = pressed: D2 at the motor end, D3 at the
/// idler end.
fn levels(d2: u8, d3: u8) -> Reading {
    Reading::Levels(Switches { d2, d3 })
}

/// A session whose span is known, 59,828 pulses: a trip at the idler end, then
/// at the motor end, where the cart rests.
fn spanned() -> Session {
    let mut session = Session::new();
    run(&mut session, Travel::Idler, 200, "tripped", 100);
    run(&mut session, Travel::Motor, 200, "tripped", 59_828);
    session
}

#[test]
fn given_a_new_page_when_one_end_trips_then_the_span_is_unknown() {
    // Given
    let mut session = Session::new();

    // When
    run(&mut session, Travel::Idler, 200, "tripped", 29_920);

    // Then
    assert_eq!(session.span, None);
}

#[test]
fn given_a_trip_at_one_end_when_the_other_end_trips_then_its_pulses_are_the_span() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Idler, 200, "tripped", 29_920);

    // When
    run(&mut session, Travel::Motor, 200, "tripped", 59_828);

    // Then
    assert_eq!(session.span, Some(59_828));
}

#[test]
fn given_a_trip_at_one_end_when_the_same_end_trips_again_then_no_span_is_measured() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Idler, 200, "tripped", 100);

    // When
    run(&mut session, Travel::Idler, 200, "tripped", 200);

    // Then
    assert_eq!(session.span, None);
}

#[test]
fn given_a_known_span_when_a_move_times_out_then_the_span_and_the_trip_are_forgotten() {
    // Given
    let mut session = spanned();

    // When
    run(&mut session, Travel::Idler, 408, "timed_out", 0);

    // Then
    assert_eq!(session.span, None);
    assert_eq!(session.last_trip, None);
}

#[test]
fn given_a_known_span_when_a_moves_answer_is_lost_then_the_span_and_the_trip_are_forgotten() {
    // Given
    let mut session = spanned();

    // When
    run(&mut session, Travel::Idler, 408, "no_answer", 0);

    // Then
    assert_eq!(session.span, None);
    assert_eq!(session.last_trip, None);
}

#[test]
fn given_a_known_span_when_an_answer_says_nothing_moved_then_nothing_changes() {
    // Given
    let results = ["already_there", "bad_speed", "refused", "busy"];

    // When
    let after: Vec<_> = results
        .iter()
        .map(|result| {
            let mut session = spanned();
            run(&mut session, Travel::Motor, 409, result, 0);
            (session.span, session.last_trip)
        })
        .collect();

    // Then
    assert!(
        after
            .iter()
            .all(|&state| state == (Some(59_828), Some(Travel::Motor))),
        "{after:?}"
    );
}

#[test]
fn given_a_known_span_when_a_move_is_stopped_then_the_span_stays_but_the_switch_is_lost() {
    // Given
    let mut session = spanned();

    // When
    run(&mut session, Travel::Idler, 409, "stopped", 1_000);

    // Then
    assert_eq!(session.span, Some(59_828));
    assert_eq!(session.last_trip, None);
    assert!(session.next_step().contains("Find either end"));
}

#[test]
fn given_a_new_page_when_a_seek_trips_its_end_then_the_cart_can_be_measured_there() {
    // Given
    let mut session = Session::new();

    // When
    run(&mut session, Travel::Motor, 200, "tripped", 100);

    // Then
    assert_eq!(session.stop_here(), Some(Travel::Motor));
}

#[test]
fn given_the_cart_at_an_end_when_a_centring_is_refused_then_it_can_still_be_measured_there() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Motor, 200, "tripped", 100);

    // When
    run(&mut session, Travel::Centre, 400, "bad_speed", 0);

    // Then
    assert_eq!(session.stop_here(), Some(Travel::Motor));
}

#[test]
fn given_the_cart_at_an_end_when_a_move_is_stopped_then_it_can_be_measured_nowhere() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Motor, 200, "tripped", 100);

    // When
    run(&mut session, Travel::Idler, 409, "stopped", 100);

    // Then
    assert_eq!(session.stop_here(), None);
}

#[test]
fn given_a_known_span_when_the_cart_is_centred_then_it_can_be_measured_at_the_centre() {
    // Given
    let mut session = spanned();

    // When
    run(&mut session, Travel::Centre, 200, "centred", 29_911);

    // Then
    assert_eq!(session.stop_here(), Some(Travel::Centre));
}

#[test]
fn given_a_refreshed_page_when_the_servers_state_is_restored_then_it_picks_up_from_there() {
    // Given
    let mut session = Session::new();

    // When
    session.restore(Some(Travel::Idler), Some(59_822));

    // Then
    assert_eq!(session.stop_here(), Some(Travel::Idler));
    assert!(session.next_step().contains("59,822"));
}

#[test]
fn given_a_stop_after_a_centring_when_neither_switch_is_pressed_then_the_cart_is_not_known() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Centre, 200, "centred", 29_910);
    run(&mut session, Travel::Idler, 409, "stopped", 1_000);

    // When
    session.read(levels(0, 0), "now".to_string());

    // Then
    assert_eq!(session.place(), Place::Unknown);
}

#[test]
fn given_a_move_under_way_when_stop_is_sent_then_the_status_line_warns_of_it() {
    // Given
    let mut session = Session::new();
    assert!(session.begin(Travel::Motor, 0.0));

    // When
    session.stop_asked(None);

    // Then
    let (tone, line) = session.status(0.0);
    assert_eq!(tone, Tone::Warn);
    assert!(line.contains("Stop sent"));
}

#[test]
fn given_a_stop_sent_when_the_move_ends_then_the_status_line_no_longer_says_so() {
    // Given
    let mut session = Session::new();
    assert!(session.begin(Travel::Motor, 0.0));
    session.stop_asked(None);

    // When
    session.finish(answer(409, "stopped", 10), 1.0, "now".to_string());

    // Then
    assert!(!session.status(0.0).1.contains("Stop sent"));
}

#[test]
fn given_no_move_when_stop_is_sent_then_the_status_line_says_nothing_of_it() {
    // Given
    let mut session = Session::new();

    // When
    session.stop_asked(None);

    // Then
    assert!(!session.status(0.0).1.contains("Stop sent"));
}

#[test]
fn given_a_trip_at_one_end_when_the_other_trips_on_the_wrong_switch_then_it_is_no_trip() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Idler, 200, "tripped", 100);

    // When
    run(&mut session, Travel::Motor, 409, "tripped", 50);

    // Then
    assert_eq!(session.span, None);
    assert_eq!(session.last_trip, None);
}

#[test]
fn given_a_centring_when_neither_switch_is_pressed_then_the_cart_is_at_the_centre() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Centre, 200, "centred", 29_910);

    // When
    session.read(levels(0, 0), "now".to_string());

    // Then
    assert_eq!(session.place(), Place::Centre);
}

#[test]
fn given_a_centring_when_the_motor_switch_reads_pressed_then_the_cart_is_at_the_motor() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Centre, 200, "centred", 29_910);

    // When
    session.read(levels(1, 0), "now".to_string());

    // Then
    assert_eq!(session.place(), Place::Motor);
}

#[test]
fn given_a_switch_pressed_after_a_centring_when_it_is_released_then_the_cart_is_not_known() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Centre, 200, "centred", 29_910);
    session.read(levels(1, 0), "now".to_string());

    // When
    session.read(levels(0, 0), "now".to_string());

    // Then
    assert_eq!(session.place(), Place::Unknown);
}

#[test]
fn given_a_new_page_when_both_switches_read_pressed_then_the_cart_is_at_no_place() {
    // Given
    let mut session = Session::new();

    // When
    session.read(levels(1, 1), "now".to_string());

    // Then
    assert_eq!(session.place(), Place::BothPressed);
}

#[test]
fn given_a_move_under_way_when_a_reading_lands_then_it_is_ignored() {
    // Given
    let mut session = Session::new();
    assert!(session.begin(Travel::Motor, 0.0));

    // When
    session.read(levels(1, 0), "now".to_string());

    // Then
    assert_eq!(session.switches(), None);
    assert_eq!(session.place(), Place::Moving);
}

#[test]
fn given_a_reading_when_a_move_starts_then_the_reading_is_dropped() {
    // Given
    let mut session = Session::new();
    session.read(levels(0, 0), "now".to_string());

    // When
    let started = session.begin(Travel::Motor, 0.0);

    // Then
    assert!(started);
    assert_eq!(session.switches(), None);
}

#[test]
fn given_a_move_under_way_when_a_second_move_is_asked_for_then_it_does_not_start() {
    // Given
    let mut session = Session::new();
    assert!(session.begin(Travel::Motor, 0.0));

    // When
    let started = session.begin(Travel::Idler, 0.0);

    // Then
    assert!(!started);
}

#[test]
fn given_a_move_from_another_page_when_a_move_is_asked_for_then_it_does_not_start() {
    // Given
    let mut session = Session::new();
    run(&mut session, Travel::Motor, 200, "tripped", 1);
    session.read(Reading::MovingElsewhere, "now".to_string());

    // When
    let started = session.begin(Travel::Idler, 0.0);

    // Then
    assert!(!started);
}

#[test]
fn given_the_cart_on_the_motor_switch_when_a_centring_starts_then_it_heads_left() {
    // Given
    let mut session = Session::new();
    session.read(levels(1, 0), "now".to_string());

    // When
    let started = session.begin(Travel::Centre, 0.0);

    // Then
    assert!(started);
    assert_eq!(session.heading(), Some(Heading::Left));
}
