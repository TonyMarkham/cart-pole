use serde::Deserialize;

/// `[calibration]`: where `calibrate_service.py` listens, and how to talk to
/// it. `settings.toml` explains each value where it is set.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CalibrationSettings {
    pub(crate) service: String,
    pub(crate) timeout_ms: u64,
    pub(crate) poll_ms: u64,
    pub(crate) refresh_timeout_ms: u64,
}
