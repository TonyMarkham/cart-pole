pub(crate) mod arduino_settings;
pub(crate) mod brain_settings;
pub(crate) mod camera_settings;
pub(crate) mod home;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    error::app_result::AppResult,
    settings::{
        arduino_settings::ArduinoSettings, brain_settings::BrainSettings,
        camera_settings::CameraSettings,
    },
};

use serde::Deserialize;
use std::fs;

/// The brain's tables in `settings.toml`, the file it shares with the
/// webserver: `[arduino]`, where the board is; `[brain]`, its own; `[camera]`,
/// the sensor's mode, shutter and gain. The webserver's tables are left to it,
/// so unknown tables are allowed here; inside each table a key the brain
/// doesn't know stops the start, so a misspelt key is never silently ignored.
#[derive(Debug, Deserialize)]
pub(crate) struct Settings {
    pub(crate) arduino: ArduinoSettings,
    pub(crate) brain: BrainSettings,
    pub(crate) camera: CameraSettings,
}

impl Settings {
    /// Reads and checks the file at `path`.
    pub(crate) fn load(path: &str) -> AppResult<Self> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }
}
