use crate::history::History;

#[test]
fn given_a_new_history_when_read_then_it_is_zeros() {
    // Given
    let history = History::new(6);

    // When
    let actions: Vec<f64> = history.actions().collect();

    // Then
    assert_eq!(actions, vec![0.0; 6]);
}

#[test]
fn given_two_commands_when_read_then_the_newest_is_first_and_the_zeros_follow() {
    // Given
    let mut history = History::new(6);

    // When
    history.push(0.25);
    history.push(-0.5);

    // Then
    let actions: Vec<f64> = history.actions().collect();
    assert_eq!(actions, vec![-0.5, 0.25, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn given_seven_commands_when_read_then_only_the_last_six_remain() {
    // Given
    let mut history = History::new(6);

    // When
    for k in 1..=7 {
        history.push(f64::from(k) / 10.0);
    }

    // Then
    let actions: Vec<f64> = history.actions().collect();
    assert_eq!(actions, vec![0.7, 0.6, 0.5, 0.4, 0.3, 0.2]);
}

#[test]
fn given_a_policy_with_no_past_commands_when_one_is_pushed_then_the_history_stays_empty() {
    // Given
    let mut history = History::new(0);

    // When
    history.push(0.9);

    // Then
    assert_eq!(history.actions().count(), 0);
}
