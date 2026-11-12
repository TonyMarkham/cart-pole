use crate::{
    dataset::marker_names::{MARKERS_FILE, read_marker_names},
    error::{app_error::AppError, app_result::AppResult},
    players::run_result::RunResult,
};

use std::{
    fs,
    path::{Path, PathBuf},
};

/// The file training is pointed at, in the output folder beside the runs
/// (`data=../datasets/generated/data.yaml`, __object_detection__/train's
/// README).
pub(crate) const DATA_YAML_FILE: &str = "data.yaml";

/// Every frame is greyscale, one channel (GreyscaleEncoder in
/// simulation.cart-pole.synthetic-data-gen writes BT.601 luma).
const CHANNELS: u32 = 1;

/// data.yaml's keys, as Ultralytics reads a dataset yaml: the train and val
/// image folders, the class names (entry i is class i), and the channel
/// count.
const TRAIN_KEY: &str = "train";
const VAL_KEY: &str = "val";
const NAMES_KEY: &str = "names";
const CHANNELS_KEY: &str = "channels";

/// YAML's block layout: a key ends with `: ` before an inline value, or with
/// `:` and a line break before a list; each list item is a line starting
/// `- `.
const KEY_VALUE_SEPARATOR: &str = ": ";
const KEY_LIST_END: &str = ":\n";
const LIST_ITEM_PREFIX: &str = "- ";
const LINE_END: char = '\n';

/// Writes `output_dir/data.yaml` for `runs`, replacing any earlier one, and
/// returns its path. Each run must sit directly in `output_dir` (its
/// markers.json is read from there), and every run's markers.json must name
/// the same classes in the same order: one data.yaml has one `names` list,
/// and a run labelled with other class ids would train the wrong classes.
pub(crate) fn write_data_yaml(output_dir: &Path, runs: &[RunResult]) -> AppResult<PathBuf> {
    let mut train = Vec::new();
    let mut val = Vec::new();
    let mut names: Option<(String, Vec<String>)> = None;
    for run in runs {
        let folder = run
            .run_directory
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                AppError::dataset(format!(
                    "set '{}': run folder {} has no usable name",
                    run.set_name,
                    run.run_directory.display()
                ))
            })?
            .to_owned();
        let run_names = read_marker_names(&output_dir.join(&folder))?;
        match &names {
            None => names = Some((folder.clone(), run_names)),
            Some((first_folder, first_names)) if *first_names != run_names => {
                return Err(AppError::dataset(format!(
                    "{folder}/{MARKERS_FILE} names {run_names:?} but \
                     {first_folder}/{MARKERS_FILE} names {first_names:?}"
                )));
            }
            Some(_) => {}
        }
        if run.is_val {
            val.push(folder);
        } else {
            train.push(folder);
        }
    }
    let Some((_, names)) = names else {
        return Err(AppError::dataset("no runs to list in data.yaml"));
    };
    let path = output_dir.join(DATA_YAML_FILE);
    fs::write(&path, data_yaml_text(&train, &val, &names))
        .map_err(|error| AppError::io(format!("writing {}: {error}", path.display())))?;
    Ok(path)
}

/// data.yaml's text: the train and val run folders (relative to the yaml's
/// own folder, since it has no `path:` key), the class names in class order,
/// and the channel count. Example, one run each and two classes:
///
/// ```text
/// train:
/// - run_Set-01_20261112_101500
/// val:
/// - run_Set-16_20261112_103000
/// names:
/// - Left
/// - Right
/// channels: 1
/// ```
pub(crate) fn data_yaml_text(train: &[String], val: &[String], names: &[String]) -> String {
    let mut text = String::new();
    push_list(&mut text, TRAIN_KEY, train);
    push_list(&mut text, VAL_KEY, val);
    push_list(&mut text, NAMES_KEY, names);
    text.push_str(CHANNELS_KEY);
    text.push_str(KEY_VALUE_SEPARATOR);
    text.push_str(&CHANNELS.to_string());
    text.push(LINE_END);
    text
}

/// `key:` then one `- item` line per item.
fn push_list(text: &mut String, key: &str, items: &[String]) {
    text.push_str(key);
    text.push_str(KEY_LIST_END);
    for item in items {
        text.push_str(LIST_ITEM_PREFIX);
        text.push_str(item);
        text.push(LINE_END);
    }
}
