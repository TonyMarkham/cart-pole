use crate::{
    constants::FRAMES_FILE,
    error::{app_error::AppError, app_result::AppResult},
    log::{civil::folder_name, frame_line::FrameLine},
};

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

/// One run's log: its folder, made at the Run press, and `frames.jsonl` in it,
/// one line per frame, flushed as each is written so a crash keeps every line
/// before it (`__documentation__/real-loop-dia.md`, *The log*).
pub(crate) struct RunLog {
    pub(crate) dir: PathBuf,
    file: BufWriter<File>,
}

impl RunLog {
    /// Makes the run's folder under `runs_dir`, named by `pressed`, and opens
    /// its file. A second Run in the same second would share the name, so the
    /// folder must not exist yet.
    pub(crate) fn open(runs_dir: &Path, pressed: SystemTime) -> AppResult<RunLog> {
        let dir = runs_dir.join(folder_name(pressed));
        fs::create_dir_all(runs_dir)?;
        fs::create_dir(&dir).map_err(|error| AppError::Io {
            message: format!("making the run's folder {}: {error}", dir.display()),
            location: error_location::ErrorLocation::from(std::panic::Location::caller()),
        })?;
        let file = BufWriter::new(File::create(dir.join(FRAMES_FILE))?);
        Ok(RunLog { dir, file })
    }

    /// One frame's line, written and flushed.
    pub(crate) fn write(&mut self, line: &FrameLine<'_>) -> AppResult<()> {
        serde_json::to_writer(&mut self.file, line)?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        Ok(())
    }
}
