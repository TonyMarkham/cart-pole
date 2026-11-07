pub(crate) mod arduino_settings;
pub(crate) mod calibration_settings;
pub(crate) mod server_settings;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    error::app_result::AppResult,
    settings::{
        arduino_settings::ArduinoSettings, calibration_settings::CalibrationSettings,
        server_settings::ServerSettings,
    },
};

use serde::Deserialize;
use std::fs;

/// The webserver's `settings.toml`, one table per part. Read once, when the
/// server starts, so a change takes effect at the next start. A key the file
/// doesn't know stops the start, so a misspelt key is never silently ignored.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Settings {
    pub(crate) server: ServerSettings,
    pub(crate) arduino: ArduinoSettings,
    pub(crate) calibration: CalibrationSettings,
}

impl Settings {
    /// Reads and checks the file at `path`.
    pub(crate) fn load(path: &str) -> AppResult<Self> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }
}
