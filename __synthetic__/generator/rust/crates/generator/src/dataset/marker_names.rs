use crate::{
    dataset::markers_file::MarkersFile,
    error::{app_error::AppError, app_result::AppResult},
};

use std::{fs, path::Path};

/// The file each run holds at its root, naming its classes (DatasetWriter's
/// `WriteMarkersJson`).
pub(crate) const MARKERS_FILE: &str = "markers.json";

/// The class names in `run_dir/markers.json`, in class order.
pub(crate) fn read_marker_names(run_dir: &Path) -> AppResult<Vec<String>> {
    let path = run_dir.join(MARKERS_FILE);
    let text = fs::read_to_string(&path)
        .map_err(|error| AppError::io(format!("reading {}: {error}", path.display())))?;
    marker_names(&text, &path)
}

/// The class names in a markers.json's text, in class order: entry i of the
/// result is class i's label. The class ids must be exactly 0, 1, ... with no
/// gap or repeat, in any order in the file, since a YOLO class is its
/// position in data.yaml's `names`. Example: ids 1, 0 with labels Right, Left
/// give [Left, Right]; ids 0, 2 fail (class 1 has no name). `source` names
/// the file in errors.
pub(crate) fn marker_names(text: &str, source: &Path) -> AppResult<Vec<String>> {
    let file: MarkersFile = serde_json::from_str(text)
        .map_err(|error| AppError::dataset(format!("parsing {}: {error}", source.display())))?;
    let mut markers = file.markers;
    markers.sort_by_key(|marker| marker.class_id);
    let mut names = Vec::with_capacity(markers.len());
    for (expected, marker) in (0u32..).zip(markers) {
        if marker.class_id != expected {
            return Err(AppError::dataset(format!(
                "{}: class {expected} has no marker (the next class id is {})",
                source.display(),
                marker.class_id
            )));
        }
        names.push(marker.label);
    }
    if names.is_empty() {
        return Err(AppError::dataset(format!(
            "{}: no markers, so the dataset has no classes",
            source.display()
        )));
    }
    Ok(names)
}
