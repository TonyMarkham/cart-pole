use crate::settings::{
    apply_settings::ApplySettings, board_settings::BoardSettings, r3_settings::R3Settings,
};

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

/// `apply-host.toml`, whole. Every key is required: a missing or misspelt one
/// stops the run before anything is sent.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostSettings {
    /// `[r3]`: the timing instrument's line.
    pub(crate) r3: R3Settings,
    /// `[r4]`: the board under test's line.
    pub(crate) r4: BoardSettings,
    /// `[apply]`: the test.
    pub(crate) apply: ApplySettings,
}

impl HostSettings {
    pub(crate) fn read(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}
