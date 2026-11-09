use serde::Deserialize;

/// `[r4]`: the R4's USB port, and how long its WiFi join may take.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoardSettings {
    /// The R4's USB port, such as `COM3`.
    pub(crate) port: String,
    /// The line's speed, bits per second: the firmware's `config/usb.toml`
    /// `baud`.
    pub(crate) baud: u32,
    /// How long to keep asking `STATUS` until the R4 says it is running, ms.
    pub(crate) status_timeout_ms: u64,
}
