use serde::Deserialize;

/// `[run]`: where the brain listens, and how long to wait for it.
/// `settings.toml` explains each value where it is set.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunSettings {
    pub(crate) brain: String,
    pub(crate) timeout_ms: u64,
}
