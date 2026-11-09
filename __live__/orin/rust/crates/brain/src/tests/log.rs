use crate::log::civil::folder_name;

use std::time::{Duration, UNIX_EPOCH};

/// Each moment's epoch seconds were worked out with Python's `datetime`, in
/// UTC, before the test was written.
#[test]
fn given_the_trainers_run_moment_when_named_then_it_is_the_trainers_folder_name() {
    // Given: 2026-11-01 16:34:58 UTC
    let moment = UNIX_EPOCH + Duration::from_secs(1_793_550_898);

    // When
    let name = folder_name(moment);

    // Then
    assert_eq!(name, "20261101_163458");
}

#[test]
fn given_a_leap_day_when_named_then_the_date_is_right() {
    // Given: 2024-02-29 12:00:00 UTC
    let moment = UNIX_EPOCH + Duration::from_secs(1_709_208_000);

    // When
    let name = folder_name(moment);

    // Then
    assert_eq!(name, "20240229_120000");
}

#[test]
fn given_the_epoch_when_named_then_it_is_the_first_moment_of_1970() {
    assert_eq!(folder_name(UNIX_EPOCH), "19700101_000000");
}

#[test]
fn given_the_first_of_march_2000_when_named_then_the_centurys_leap_year_is_right() {
    // Given: 2000-03-01 00:00:00 UTC, the day after a leap day that only
    // comes every 400 years
    let moment = UNIX_EPOCH + Duration::from_secs(951_868_800);

    // When
    let name = folder_name(moment);

    // Then
    assert_eq!(name, "20000301_000000");
}
