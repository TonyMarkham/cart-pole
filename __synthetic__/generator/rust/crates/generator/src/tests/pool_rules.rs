//! The rules a pool of sets must meet before anything runs.

use crate::{pool::check::check, tests::fixture::entry};

#[test]
fn given_one_train_and_one_val_set_when_checked_then_the_pool_is_accepted() {
    // given
    let pool = [entry("A", false), entry("B", true)];
    let val = ["B.json".to_owned()];

    // when
    let result = check(&pool, &val);

    // then
    assert!(result.is_ok());
}

#[test]
fn given_two_val_sets_when_checked_then_the_pool_is_accepted() {
    // given
    let pool = [entry("A", false), entry("B", true), entry("C", true)];
    let val = ["B.json".to_owned(), "C.json".to_owned()];

    // when
    let result = check(&pool, &val);

    // then
    assert!(result.is_ok());
}

#[test]
fn given_a_val_name_not_in_the_pool_when_checked_then_the_pool_is_rejected() {
    // given: val names a file the sets folder does not have
    let pool = [entry("A", false), entry("B", true)];
    let val = ["B.json".to_owned(), "Set-99.json".to_owned()];

    // when
    let result = check(&pool, &val);

    // then
    assert!(result.is_err());
}

#[test]
fn given_two_sets_with_one_name_when_checked_then_the_pool_is_rejected() {
    // given
    let pool = [entry("A", false), entry("A", true)];
    let val = ["A.json".to_owned()];

    // when
    let result = check(&pool, &val);

    // then
    assert!(result.is_err());
}

#[test]
fn given_no_val_set_when_checked_then_the_pool_is_rejected() {
    // given
    let pool = [entry("A", false), entry("B", false)];
    let val: [String; 0] = [];

    // when
    let result = check(&pool, &val);

    // then
    assert!(result.is_err());
}

#[test]
fn given_no_train_set_when_checked_then_the_pool_is_rejected() {
    // given
    let pool = [entry("A", true), entry("B", true)];
    let val = ["A.json".to_owned(), "B.json".to_owned()];

    // when
    let result = check(&pool, &val);

    // then
    assert!(result.is_err());
}
