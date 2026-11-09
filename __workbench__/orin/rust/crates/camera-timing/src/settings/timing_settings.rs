use crate::settings::{
    camera_settings::CameraSettings, led_settings::LedSettings, test_settings::TestSettings,
};

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

/// `camera-timing.toml`, whole. Every key is required: a missing or misspelt
/// one stops the run before the camera opens.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TimingSettings {
    /// `[camera]`: the brain's camera values.
    pub(crate) camera: CameraSettings,
    /// `[led]`: the LED's GPIO line.
    pub(crate) led: LedSettings,
    /// `[test]`: the flashes and what counts as lit.
    pub(crate) test: TestSettings,
}

impl TimingSettings {
    pub(crate) fn read(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}
