//! Laying a set file over the master (`GenerationSetBuilder::overlay`): only
//! the keys the file gives change, and each replaces the master's whole.

use crate::{
    pool::{generation_set_builder::GenerationSetBuilder, parse_set::parse_set},
    proto::GenerationSet,
    tests::fixture::{SET, approx},
};

fn master() -> GenerationSet {
    parse_set(SET).unwrap()
}

fn merge(set_file: &str) -> GenerationSet {
    GenerationSetBuilder::new(master())
        .overlay(parse_set(set_file).unwrap())
        .build()
}

#[test]
fn given_a_set_file_with_only_a_seed_when_merged_then_only_the_seed_changes() {
    // given
    let set_file = r#"{"seed": 2002}"#;

    // when
    let set = merge(set_file);

    // then
    let master = master();
    assert_eq!(set.seed, 2002);
    assert_eq!(set.name, master.name);
    assert_eq!(set.states, master.states);
    assert_eq!(set.desks, master.desks);
    assert_eq!(set.fixed_poses.len(), master.fixed_poses.len());
    assert!(set.max_z.is_some_and(|max_z| approx(max_z, 2.5)));
}

#[test]
fn given_an_empty_set_file_when_merged_then_the_set_is_the_master() {
    // given
    let set_file = "{}";

    // when
    let set = merge(set_file);

    // then
    let master = master();
    assert_eq!(set.name, master.name);
    assert_eq!(set.seed, master.seed);
    assert_eq!(set.desks, master.desks);
}

#[test]
fn given_a_set_file_with_a_list_when_merged_then_the_list_replaces_the_masters() {
    // given: the master has two desks
    let set_file = r#"{"desks": ["Desk (Writing)"]}"#;

    // when
    let set = merge(set_file);

    // then: one desk, not three
    assert_eq!(set.desks, ["Desk (Writing)"]);
}

#[test]
fn given_a_set_file_with_a_range_when_merged_then_the_range_replaces_the_masters() {
    // given: the master's camera yaw is -5..5
    let set_file = r#"{"cameraYaw": {"min": -8, "max": 8}}"#;

    // when
    let set = merge(set_file);

    // then
    assert!(
        set.camera_yaw
            .is_some_and(|range| approx(range.min, -8.0) && approx(range.max, 8.0))
    );
}

#[test]
fn given_a_set_file_with_a_name_when_merged_then_the_name_replaces_the_masters() {
    // given: the master is named Set-01
    let set_file = r#"{"name": "Set-16"}"#;

    // when
    let set = merge(set_file);

    // then
    assert_eq!(set.name, "Set-16");
}

#[test]
fn given_a_set_file_with_a_split_when_parsed_then_it_is_an_error() {
    // given: split is no longer a key (val is generator.toml's list)
    let set_file = r#"{"split": "SPLIT_VAL"}"#;

    // when
    let result = parse_set(set_file);

    // then
    assert!(result.is_err());
}

#[test]
fn given_a_misspelled_key_in_a_set_file_when_parsed_then_it_is_an_error() {
    // given
    let set_file = r#"{"sead": 2002}"#;

    // when
    let result = parse_set(set_file);

    // then
    assert!(result.is_err());
}
