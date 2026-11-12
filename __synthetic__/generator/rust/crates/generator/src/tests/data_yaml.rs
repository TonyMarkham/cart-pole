//! data.yaml: its text, and writing it from the runs on disk.

use crate::{
    dataset::{
        data_yaml::{DATA_YAML_FILE, data_yaml_text},
        marker_names::MARKERS_FILE,
        write_data_yaml,
    },
    players::run_result::RunResult,
    tests::temp_dir::TempDir,
};

use std::fs;

/// Two runs' folder names, as players name them.
const TRAIN_RUN: &str = "run_Set-01_20261112_101500";
const VAL_RUN: &str = "run_Set-16_20261112_103000";

/// A markers.json with two classes, Left and Right.
const MARKERS_LEFT_RIGHT: &str = r#"{"markers": [
    {"classId": 0, "label": "Left", "id": 376},
    {"classId": 1, "label": "Right", "id": 596}
]}"#;

/// The same classes in the other order: Right is class 0.
const MARKERS_RIGHT_LEFT: &str = r#"{"markers": [
    {"classId": 0, "label": "Right", "id": 596},
    {"classId": 1, "label": "Left", "id": 376}
]}"#;

/// `folder` in `dir` with `markers` as its markers.json, and the run that
/// reports it.
fn run(dir: &TempDir, folder: &str, is_val: bool, markers: &str) -> RunResult {
    let run_directory = dir.path().join(folder);
    fs::create_dir_all(&run_directory).unwrap();
    fs::write(run_directory.join(MARKERS_FILE), markers).unwrap();
    RunResult {
        set_name: folder.to_owned(),
        is_val,
        run_directory,
        frames: 1,
    }
}

#[test]
fn given_one_train_and_one_val_run_when_written_then_the_text_lists_both() {
    // given
    let train = [TRAIN_RUN.to_owned()];
    let val = [VAL_RUN.to_owned()];
    let names = ["Left".to_owned(), "Right".to_owned()];

    // when
    let text = data_yaml_text(&train, &val, &names);

    // then
    assert_eq!(
        text,
        "train:\n- run_Set-01_20261112_101500\n\
         val:\n- run_Set-16_20261112_103000\n\
         names:\n- Left\n- Right\n\
         channels: 1\n"
    );
}

#[test]
fn given_runs_that_agree_when_written_then_data_yaml_is_beside_them() {
    // given
    let dir = TempDir::new("agree");
    let runs = [
        run(&dir, TRAIN_RUN, false, MARKERS_LEFT_RIGHT),
        run(&dir, VAL_RUN, true, MARKERS_LEFT_RIGHT),
    ];

    // when
    let path = write_data_yaml(dir.path(), &runs).unwrap();

    // then
    assert_eq!(path, dir.path().join(DATA_YAML_FILE));
    let text = fs::read_to_string(path).unwrap();
    assert!(text.starts_with("train:\n- run_Set-01_20261112_101500\nval:\n- run_Set-16"));
}

#[test]
fn given_runs_whose_classes_differ_when_written_then_it_is_an_error() {
    // given: the val run has Left and Right swapped
    let dir = TempDir::new("disagree");
    let runs = [
        run(&dir, TRAIN_RUN, false, MARKERS_LEFT_RIGHT),
        run(&dir, VAL_RUN, true, MARKERS_RIGHT_LEFT),
    ];

    // when
    let result = write_data_yaml(dir.path(), &runs);

    // then
    assert!(result.is_err());
    assert!(!dir.path().join(DATA_YAML_FILE).exists());
}
