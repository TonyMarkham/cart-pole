use crate::{
    error::{app_error::AppError, app_result::AppResult},
    pool::parse_set::parse_set,
    proto::GenerationSet,
};

use std::{fs, path::Path};

/// One generation set's file (the master, or a set of the pool) read and
/// parsed; an error names the file.
pub(crate) fn read_set(path: &Path) -> AppResult<GenerationSet> {
    let text = fs::read_to_string(path)
        .map_err(|error| AppError::io(format!("reading {}: {error}", path.display())))?;
    parse_set(&text).map_err(|error| AppError::set(format!("parsing {}: {error}", path.display())))
}
