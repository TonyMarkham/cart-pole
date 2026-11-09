use crate::settings::{board_settings::BoardSettings, wifi_settings::WifiSettings};

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

/// `wifi-host.toml`, whole. Every key is required: a missing or misspelt one
/// stops the run before anything is sent.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostSettings {
    /// `[r4]`: the board under test's line.
    pub(crate) r4: BoardSettings,
    /// `[wifi]`: the test.
    pub(crate) wifi: WifiSettings,
}

impl HostSettings {
    pub(crate) fn read(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}
