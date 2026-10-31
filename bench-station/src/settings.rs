//! bench-station.toml, read once at start-up.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub station: Station,
    pub serial: Serial,
    pub flash: Flash,
    pub rig: Rig,
    pub experiment: Experiment,
}

#[derive(Debug, Deserialize)]
pub struct Station {
    pub listen: String,
    pub log_dir: PathBuf,
    pub page_log_lines: usize,
    pub board_page: String,
}

#[derive(Debug, Deserialize)]
pub struct Serial {
    pub port: String,
    pub baud: u32,
    pub reply_timeout_ms: u64,
    pub reopen_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct Flash {
    pub arduino_cli: String,
    pub fqbn: String,
    pub work_dir: PathBuf,
    pub port_wait_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct Rig {
    pub seek_speed: u32,
    pub accel: u32,
    pub jog_pulses: u32,
    pub settle_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct Experiment {
    pub conditions: String,
    pub trial_top: u32,
    pub trial_dt_ms: u32,
    pub j_start: u32,
    pub j_growth_percent: u32,
    pub trials_per_j: u32,
    pub noise_repeats: u32,
    pub noise_away_pulses: u32,
    pub steady_ms: u32,
    pub hold_ms: u32,
    pub end_margin_pulses: u32,
    pub resumes: u32,
}

impl Settings {
    /// Reads `path`; relative paths inside resolve against its folder.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut settings: Settings = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let folder = path.parent().unwrap_or(Path::new("."));
        settings.station.log_dir = folder.join(&settings.station.log_dir);
        settings.flash.work_dir = folder.join(&settings.flash.work_dir);
        Ok(settings)
    }
}
