use serde::Deserialize;

/// `[led]`: the LED's GPIO line.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LedSettings {
    /// The line's name as `gpioinfo` lists it; empty until it is read off the
    /// Orin.
    pub(crate) line_name: String,
}
