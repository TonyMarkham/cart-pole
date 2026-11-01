use crate::error::result::AppResult;

use serde::{Deserialize, Serialize};
use std::fs;

/// The webserver's `settings.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Settings {
    pub(crate) arduino: ArduinoSettings,
}

/// `[arduino]`: where the rig's Arduino answers, and how often to ask.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArduinoSettings {
    pub(crate) address: String,
    pub(crate) port: u16,
    pub(crate) timeout_ms: u64,
    pub(crate) poll_ms: u64,
    pub(crate) move_timeout_ms: u64,
    pub(crate) half_span_pulses: u64,
    pub(crate) stop_sends: u32,
    pub(crate) stop_gap_ms: u64,
}

impl Settings {
    pub(crate) fn load(path: &str) -> AppResult<Self> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }
}
