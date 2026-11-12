use crate::{
    error::{app_error::AppError, app_result::AppResult},
    players::player_grid::PlayerGrid,
};

use serde::Deserialize;
use std::{
    fs,
    num::{NonZeroU16, NonZeroU64},
    path::{self, Path, PathBuf},
};

/// The folder of a config path with no folder part (`--config
/// generator.toml`): the empty path, which `Path::join` treats as the
/// working directory, so relative paths in the toml resolve from there.
const CURRENT_FOLDER: &str = "";

/// generator.toml: the generator's settings. A relative path in it resolves
/// against the file's own folder, so the file works from any working
/// directory. Example: sets_dir = "../sets" in
/// __synthetic__/generator/rust/generator.toml is __synthetic__/generator/sets.
#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    /// The master set: complete, the starting point of every set in the pool.
    pub(crate) master: PathBuf,
    /// The folder of generation sets, one *.json per set.
    pub(crate) sets_dir: PathBuf,
    /// The val sets, by file name in `sets_dir` (e.g. "Set-16.json"); every
    /// other set is train.
    pub(crate) val: Vec<String>,
    /// Where the dataset lands: each player writes its run folder
    /// (`run_<set>_<timestamp>/`) here, and the generator writes data.yaml
    /// here. Made absolute at load, because it is sent to players
    /// (`GenerationJob.output_directory`) whose working directory is their
    /// own.
    pub(crate) output_dir: PathBuf,
    /// The Unity player build that runs the sets (SyntheticCamera.exe, whose
    /// scene holds both modes); launched in generator mode.
    pub(crate) player: PathBuf,
    /// The players' window grid, as the policy trainer's settings.toml
    /// names it: `instance_rows x instance_columns` players run at once
    /// (each taking the next set as it finishes one; no more launch than the
    /// pool has sets), their windows filling `screen_width x screen_height`
    /// pixels from the screen's top-left corner (`PlayerGrid`).
    pub(crate) instance_rows: NonZeroU16,
    pub(crate) instance_columns: NonZeroU16,
    pub(crate) screen_width: NonZeroU16,
    pub(crate) screen_height: NonZeroU16,
    /// Player k (from 0) listens on 127.0.0.1, port `first_port + k`.
    pub(crate) first_port: u16,
    /// How long a player may take from launch to accepting the WebSocket
    /// connection, in seconds, before the run stops.
    pub(crate) player_start_timeout_secs: NonZeroU64,
    /// How long to wait between connection attempts while a player starts,
    /// in milliseconds.
    pub(crate) connect_retry_millis: NonZeroU64,
    /// Each player's Unity log, `player_<k>.log`, goes here (without it,
    /// every player would write the same Player.log).
    pub(crate) log_dir: PathBuf,
}

impl Config {
    pub(crate) fn load(path: &Path) -> AppResult<Self> {
        let text = fs::read_to_string(path)
            .map_err(|error| AppError::io(format!("reading {}: {error}", path.display())))?;
        let mut config: Self = toml::from_str(&text)
            .map_err(|error| AppError::settings(format!("parsing {}: {error}", path.display())))?;
        let folder = path.parent().unwrap_or_else(|| Path::new(CURRENT_FOLDER));
        config.master = resolve(folder, &config.master);
        config.sets_dir = resolve(folder, &config.sets_dir);
        config.player = resolve(folder, &config.player);
        config.log_dir = resolve(folder, &config.log_dir);
        let output_dir = resolve(folder, &config.output_dir);
        config.output_dir = path::absolute(&output_dir).map_err(|error| {
            AppError::io(format!("making {} absolute: {error}", output_dir.display()))
        })?;
        // The grid is checked here, so a bad one stops the run before any
        // player launches.
        config.grid()?;
        Ok(config)
    }

    /// The players' window grid from instance_rows, instance_columns and
    /// the screen size (`PlayerGrid::new`).
    pub(crate) fn grid(&self) -> AppResult<PlayerGrid> {
        PlayerGrid::new(
            self.instance_rows,
            self.instance_columns,
            self.screen_width,
            self.screen_height,
        )
    }
}

/// `path` as written in generator.toml: unchanged when absolute, else joined
/// onto the toml's folder.
fn resolve(folder: &Path, path: &Path) -> PathBuf {
    if path.is_relative() {
        folder.join(path)
    } else {
        path.to_path_buf()
    }
}
