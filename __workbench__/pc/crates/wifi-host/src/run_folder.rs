use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// The name the settings file is copied to inside each run's folder.
const SETTINGS_COPY: &str = "settings.toml";

/// The folder `output_dir` is relative to when the settings file's path has
/// no folder of its own: the current one.
const CURRENT_FOLDER: &str = ".";

/// A new folder for one run: `<output_dir>/<test>_<seconds since 1970>`,
/// with `output_dir` taken relative to the settings file's folder, and the
/// settings file copied into it, so each run's results sit with the values
/// that made them.
pub(crate) fn create(settings_path: &Path, output_dir: &str, test: &str) -> Result<PathBuf> {
    let base = settings_path
        .parent()
        .unwrap_or_else(|| Path::new(CURRENT_FOLDER))
        .join(output_dir);
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("the PC's clock is before 1970")?
        .as_secs();
    let folder = base.join(format!("{test}_{seconds}"));
    fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    fs::copy(settings_path, folder.join(SETTINGS_COPY))
        .with_context(|| format!("copying {} into the run", settings_path.display()))?;
    Ok(folder)
}
