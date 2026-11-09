use crate::{board::travel::Travel, measure::pole::Pole};

#[test]
fn given_the_three_stops_when_their_poses_are_counted_then_there_are_seven() {
    // Given
    let stops = [Travel::Idler, Travel::Motor, Travel::Centre];

    // When
    let count: usize = stops.into_iter().map(|stop| Pole::at(stop).len()).sum();

    // Then
    assert_eq!(count, 7);
}

#[test]
fn given_the_centre_when_its_poses_are_listed_then_it_has_only_the_hanging_one() {
    // Given
    let stop = Travel::Centre;

    // When
    let poses = Pole::at(stop);

    // Then
    assert_eq!(poses, &[Pole::Hanging]);
}

#[test]
fn given_a_pose_when_its_key_is_written_then_it_is_the_services_name() {
    // Given
    let poses = [
        (Pole::Upright, Travel::Idler, "idler upright"),
        (Pole::Hanging, Travel::Centre, "centre hanging"),
        (Pole::Horizontal, Travel::Motor, "motor horizontal"),
    ];

    // When
    let keys: Vec<String> = poses
        .iter()
        .map(|&(pole, stop, _)| pole.pose_key(stop))
        .collect();

    // Then
    let expected: Vec<&str> = poses.iter().map(|&(_, _, key)| key).collect();
    assert_eq!(keys, expected);
}

#[test]
fn given_the_pole_held_level_when_named_at_each_end_then_it_points_away_from_the_centre() {
    // Given
    let pole = Pole::Horizontal;

    // When
    let at_idler = pole.name(Travel::Idler);
    let at_motor = pole.name(Travel::Motor);

    // Then
    assert!(at_idler.ends_with("left"));
    assert!(at_motor.ends_with("right"));
}
