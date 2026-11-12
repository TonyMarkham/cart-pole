use crate::{
    error::{app_error::AppError, app_result::AppResult},
    pool::pool_entry::PoolEntry,
};

use std::collections::HashSet;

/// The rules a pool must meet before anything runs:
/// - every set has a name, and no two share one (the name is the run
///   folder's: `run_<name>_<timestamp>`);
/// - every file named in generator.toml's `val` is a set in the pool (a
///   misspelled name would otherwise quietly leave val empty);
/// - at least one val and one train set, so data.yaml has both lists.
pub(crate) fn check(pool: &[PoolEntry], val: &[String]) -> AppResult<()> {
    for file_name in val {
        let in_pool = pool.iter().any(|entry| {
            entry
                .path
                .file_name()
                .is_some_and(|name| name == file_name.as_str())
        });
        if !in_pool {
            return Err(AppError::pool(format!(
                "val names {file_name}, which is not a set in the pool"
            )));
        }
    }

    let mut names = HashSet::new();
    for entry in pool {
        let set = &entry.set;
        if set.name.is_empty() {
            return Err(AppError::pool(format!(
                "{}: name is empty",
                entry.path.display()
            )));
        }
        if !names.insert(set.name.as_str()) {
            return Err(AppError::pool(format!(
                "{}: the name '{}' is already used by another set",
                entry.path.display(),
                set.name
            )));
        }
    }

    let val_count = pool.iter().filter(|entry| entry.is_val).count();
    let train_count = pool.len().saturating_sub(val_count);
    if val_count == 0 || train_count == 0 {
        return Err(AppError::pool(format!(
            "the pool has {train_count} train and {val_count} val sets; it needs at least one of each"
        )));
    }
    Ok(())
}
