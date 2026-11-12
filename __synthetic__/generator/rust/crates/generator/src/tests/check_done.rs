//! Which `GenerationDone` replies count as a finished run.

use crate::{players::check_done::check_done, proto::GenerationDone, tests::fixture::entry};

use std::path::PathBuf;

/// The run folder every reply below names.
const RUN_DIRECTORY: &str = "D:/temp/generated/run_Set-01_20261112_101500";

/// The fixture set's `states`.
const STATES: u32 = 1000;

#[test]
fn given_a_whole_run_when_checked_then_it_is_accepted() {
    // given
    let set = entry("Set-01", true);
    let done = GenerationDone {
        run_directory: RUN_DIRECTORY.to_owned(),
        frames: STATES,
        error: String::new(),
    };

    // when
    let result = check_done(&set, &done).unwrap();

    // then
    assert_eq!(result.set_name, "Set-01");
    assert!(result.is_val);
    assert_eq!(result.run_directory, PathBuf::from(RUN_DIRECTORY));
    assert_eq!(result.frames, STATES);
}

#[test]
fn given_an_error_when_checked_then_the_run_is_rejected() {
    // given: the player refused the job
    let set = entry("Set-01", false);
    let done = GenerationDone {
        run_directory: String::new(),
        frames: 0,
        error: "desk 'Standard' is not in catalog 'Catalog'".to_owned(),
    };

    // when
    let result = check_done(&set, &done);

    // then
    assert!(result.is_err());
}

#[test]
fn given_fewer_frames_than_states_when_checked_then_the_run_is_rejected() {
    // given: no error, but one frame short
    let set = entry("Set-01", false);
    let done = GenerationDone {
        run_directory: RUN_DIRECTORY.to_owned(),
        frames: STATES - 1,
        error: String::new(),
    };

    // when
    let result = check_done(&set, &done);

    // then
    assert!(result.is_err());
}

#[test]
fn given_no_run_folder_when_checked_then_the_run_is_rejected() {
    // given
    let set = entry("Set-01", false);
    let done = GenerationDone {
        run_directory: String::new(),
        frames: STATES,
        error: String::new(),
    };

    // when
    let result = check_done(&set, &done);

    // then
    assert!(result.is_err());
}
