use serde::Deserialize;

/// `[test]`: how many frames, and where the results go, each explained in
/// `stage-timing.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestSettings {
    /// Passes of the loop timed, one frame each.
    pub(crate) frames: u32,
    /// How often a progress line is printed, in frames; 0 for never.
    pub(crate) progress_every: u32,
    /// Where each run's folder goes, relative to the settings file's folder.
    pub(crate) output_dir: String,
}
