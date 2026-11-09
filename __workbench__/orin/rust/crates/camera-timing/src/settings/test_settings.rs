use serde::Deserialize;

/// `[test]`: the flashes and what counts as lit. Each value's reason is
/// beside it in `camera-timing.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestSettings {
    /// Frames with the LED off, then on, to find it and its two levels.
    pub(crate) calibration_frames: u32,
    /// The patch's half-width around the LED, pixels.
    pub(crate) patch_half: usize,
    /// The least lit-over-dark difference the run accepts, grey levels.
    pub(crate) min_contrast: f64,
    /// Flashes, each one's length, µs, and the frames between them.
    pub(crate) pulses: u32,
    pub(crate) pulse_us: u64,
    pub(crate) pulse_spacing_frames: u32,
    /// How often a progress line is printed, in flashes; 0 for never.
    pub(crate) progress_every: u32,
    /// The longest wait for a frame, ms.
    pub(crate) frame_timeout_ms: u64,
    /// The random draws' seed.
    pub(crate) seed: u64,
    /// Where each run's folder goes, relative to the settings file's folder.
    pub(crate) output_dir: String,
}
