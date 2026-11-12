//! Reading the class names from a run's markers.json.

use crate::dataset::marker_names::marker_names;

use std::path::Path;

/// What every error below names as the file.
const SOURCE: &str = "run_Set-01/markers.json";

#[test]
fn given_markers_out_of_order_when_read_then_names_are_in_class_order() {
    // given: DatasetWriter's layout, classes listed 1 then 0
    let json = r#"{"markers": [
        {"classId": 1, "label": "Right", "id": 596},
        {"classId": 0, "label": "Left", "id": 376}
    ]}"#;

    // when
    let names = marker_names(json, Path::new(SOURCE)).unwrap();

    // then
    assert_eq!(names, ["Left", "Right"]);
}

#[test]
fn given_a_gap_in_the_class_ids_when_read_then_it_is_an_error() {
    // given: class 1 is missing
    let json = r#"{"markers": [
        {"classId": 0, "label": "Left", "id": 376},
        {"classId": 2, "label": "Pivot", "id": 920}
    ]}"#;

    // when
    let names = marker_names(json, Path::new(SOURCE));

    // then
    assert!(names.is_err());
}

#[test]
fn given_no_markers_when_read_then_it_is_an_error() {
    // given
    let json = r#"{"markers": []}"#;

    // when
    let names = marker_names(json, Path::new(SOURCE));

    // then
    assert!(names.is_err());
}
