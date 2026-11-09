use crate::settings::{
    camera_settings::CameraSettings, run_settings::RunSettings, send_settings::SendSettings,
    test_settings::TestSettings, worker_settings::WorkerSettings,
};

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

/// `stage-timing.toml`, every table; a key it doesn't know stops the start,
/// so a misspelt key is never silently ignored.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StageSettings {
    pub(crate) camera: CameraSettings,
    pub(crate) worker: WorkerSettings,
    pub(crate) run: RunSettings,
    pub(crate) send: SendSettings,
    pub(crate) test: TestSettings,
}

impl StageSettings {
    pub(crate) fn read(path: &Path) -> Result<StageSettings> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("reading {}", path.display()))
    }
}
