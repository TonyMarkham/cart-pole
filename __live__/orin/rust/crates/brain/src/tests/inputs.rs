use crate::{geometry::point::Point, history::History, inputs::Inputs, pool::Pool};

#[test]
fn given_the_first_frame_when_the_inputs_are_built_then_they_are_the_pool_in_seconds_then_the_actions()
 {
    // Given: the pre-warmed pool at 40 ms gaps, the first frame pushed, and
    // one past command
    let mut pool = Pool::pre_warmed(40.0);
    pool.push(
        1_000_000_000,
        Point { x: 0.1, z: 0.082 },
        Point { x: 0.1, z: -0.368 },
    );
    let mut history = History::new(6);
    history.push(0.5);

    // When
    let inputs = Inputs::build(&pool, &history, 1_000.0);

    // Then
    let values = inputs.values();
    assert_eq!(values.len(), 36);
    // The newest entry: age 0 s, Pivot, End.
    assert_eq!(&values[..5], &[0.0, 0.1, 0.082, 0.1, -0.368]);
    // The pre-warmed entries: ages in seconds, no detection.
    assert_eq!(&values[5..10], &[-0.04, -1.0, -1.0, -1.0, -1.0]);
    assert!((values[25] - -0.2).abs() < 1e-12);
    // The actions, newest first, zeros before the first.
    assert_eq!(&values[30..], &[0.5, 0.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn given_another_age_unit_when_the_inputs_are_built_then_only_the_ages_change() {
    // Given
    let pool = Pool::pre_warmed(50.0);
    let history = History::new(6);

    // When
    let in_seconds = Inputs::build(&pool, &history, 1_000.0);
    let in_tenths = Inputs::build(&pool, &history, 100.0);

    // Then
    assert!((in_seconds.values()[0] - -0.05).abs() < 1e-12);
    assert!((in_tenths.values()[0] - -0.5).abs() < 1e-12);
    assert_eq!(in_seconds.values()[1..5], in_tenths.values()[1..5]);
}
