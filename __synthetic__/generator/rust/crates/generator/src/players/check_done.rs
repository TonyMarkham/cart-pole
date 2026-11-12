use crate::{
    error::{app_error::AppError, app_result::AppResult},
    players::run_result::RunResult,
    pool::pool_entry::PoolEntry,
    proto::GenerationDone,
};

use std::path::PathBuf;

/// The run a player reported for `entry`'s set, accepted only when it is
/// whole: no `error`, a run folder, and exactly the set's `states` frames
/// (the player's error names the cause otherwise; a frame count that differs
/// without one would still be a different dataset than the set describes).
pub(crate) fn check_done(entry: &PoolEntry, done: &GenerationDone) -> AppResult<RunResult> {
    let set = &entry.set;
    if !done.error.is_empty() {
        let left = if done.run_directory.is_empty() {
            String::new()
        } else {
            format!(
                " (it left {} frames in {})",
                done.frames, done.run_directory
            )
        };
        return Err(AppError::player(format!(
            "set '{}' ({}): {}{left}",
            set.name,
            entry.path.display(),
            done.error
        )));
    }
    if done.run_directory.is_empty() {
        return Err(AppError::player(format!(
            "set '{}': the player reported no error and no run folder",
            set.name
        )));
    }
    if done.frames != set.states {
        return Err(AppError::player(format!(
            "set '{}': the player wrote {} frames to {}, but the set has {} states",
            set.name, done.frames, done.run_directory, set.states
        )));
    }
    Ok(RunResult {
        set_name: set.name.clone(),
        is_val: entry.is_val,
        run_directory: PathBuf::from(&done.run_directory),
        frames: done.frames,
    })
}
