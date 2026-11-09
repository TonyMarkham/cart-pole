use serde::{Deserialize, Serialize};

/// `[arduino]`: where the rig's Arduino answers, and how long to wait for it.
/// `settings.toml` explains each value where it is set. Sent whole to the
/// pages by `/arduino/settings`, so they show the same address and pause.
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
