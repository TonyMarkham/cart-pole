//! Reading one generation set from its JSON form.

use crate::{
    pool::parse_set::parse_set,
    tests::fixture::{SET, approx},
};

#[test]
fn given_a_complete_set_when_parsed_then_every_written_field_is_read() {
    // given
    let text = SET;

    // when
    let set = parse_set(text).unwrap();

    // then
    assert_eq!(set.name, "Set-01");
    assert_eq!(set.states, 1000);
    assert!(approx(set.fixed_poses[0].joints[1].value, -180.0));
    assert_eq!(set.fixed_poses[0].props, ["T-Square (Idler)"]);
    assert!(set.camera_yaw.is_some_and(|range| approx(range.max, 5.0)));
    assert!(set.max_z.is_some_and(|max_z| approx(max_z, 2.5)));
}

#[test]
fn given_fields_left_out_when_parsed_then_they_take_their_empty_values() {
    // given: SET writes no cameraPitch and no skyboxes
    let text = SET;

    // when
    let set = parse_set(text).unwrap();

    // then: a range that draws nothing, an empty list
    assert!(set.camera_pitch.is_none());
    assert!(set.skyboxes.is_empty());
}

#[test]
fn given_a_misspelled_field_when_parsed_then_it_is_an_error() {
    // given
    let text = SET.replace("\"states\"", "\"state\"");

    // when
    let result = parse_set(&text);

    // then
    assert!(result.is_err());
}

#[test]
fn given_a_value_of_the_wrong_type_when_parsed_then_it_is_an_error() {
    // given
    let text = SET.replace("\"states\": 1000", "\"states\": \"many\"");

    // when
    let result = parse_set(&text);

    // then
    assert!(result.is_err());
}
