use serde::Deserialize;

/// `[r3]`: the R3's USB port, and how long its sketch may take to start.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct R3Settings {
    /// The R3's USB port, such as `COM4`.
    pub(crate) port: String,
    /// The line's speed, bits per second: `apply_timer.ino`'s `SERIAL_BAUD`.
    pub(crate) baud: u32,
    /// How long to wait for the sketch's `READY` after the reset, ms.
    pub(crate) ready_timeout_ms: u64,
}
