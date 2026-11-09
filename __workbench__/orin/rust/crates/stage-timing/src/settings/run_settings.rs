use serde::Deserialize;

/// `[run]`: the loop's pace, as the brain runs it, each explained in
/// `stage-timing.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunSettings {
    /// The wait after each command before the next frame is taken, ms.
    pub(crate) delay_ms: u64,
    /// The gap between the pre-warmed pool's entries, ms.
    pub(crate) pre_warm_gap_ms: f64,
    /// The longest wait for a frame, ms.
    pub(crate) frame_timeout_ms: u64,
}
