use serde::Deserialize;

/// `[apply]`: the apply test. Each value's reason is beside it in
/// `apply-host.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplySettings {
    /// The two rates the commands alternate between, pulses/s.
    pub(crate) rate_low: u16,
    pub(crate) rate_high: u16,
    /// Commands sent before the R3 times, and while it times.
    pub(crate) warmup_datagrams: u32,
    pub(crate) datagrams: u32,
    /// The shortest and longest gap between commands, ms; each is drawn
    /// evenly between them.
    pub(crate) gap_min_ms: u64,
    pub(crate) gap_max_ms: u64,
    /// How long the R3 waits after a mark for the new rate, µs.
    pub(crate) apply_timeout_us: u32,
    /// How long the PC waits for the R3's line about each command, ms.
    pub(crate) result_timeout_ms: u64,
    /// The histogram's bin width, µs.
    pub(crate) histogram_bin_us: u32,
    /// The random draws' seed.
    pub(crate) seed: u64,
    /// Where each run's folder goes, relative to the settings file's folder.
    pub(crate) output_dir: String,
}
