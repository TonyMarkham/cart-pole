use crate::{
    error::{app_error::AppError, app_result::AppResult},
    pool::{
        check::check, generation_set_builder::GenerationSetBuilder, pool_entry::PoolEntry,
        read_set::read_set,
    },
    proto::GenerationSet,
};

use std::{fs, io, path::Path};

/// The extension of a generation set's file: one set per `*.json`.
const SET_EXTENSION: &str = "json";

/// Every generation set in `dir`: each `*.json` file directly in it (not in
/// subfolders), in file-name order, read and laid over a copy of `master`
/// (`GenerationSetBuilder::overlay`), marked val when its file name is in
/// `val` (generator.toml's list), then all checked as a pool (`check`).
/// The first bad file stops the load and is named in the error, so nothing
/// runs on a pool with a broken set.
pub(crate) fn load_pool(
    dir: &Path,
    master: &GenerationSet,
    val: &[String],
) -> AppResult<Vec<PoolEntry>> {
    let read_error = |error: io::Error| AppError::io(format!("reading {}: {error}", dir.display()));
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(read_error)? {
        let path = entry.map_err(read_error)?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == SET_EXTENSION)
        {
            paths.push(path);
        }
    }
    paths.sort();

    let mut pool = Vec::with_capacity(paths.len());
    for path in paths {
        let set = GenerationSetBuilder::new(master.clone())
            .overlay(read_set(&path)?)
            .build();
        let is_val = val.iter().any(|file_name| {
            path.file_name()
                .is_some_and(|name| name == file_name.as_str())
        });
        pool.push(PoolEntry { path, set, is_val });
    }
    check(&pool, val)?;
    Ok(pool)
}
