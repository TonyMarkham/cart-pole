use serde::Deserialize;

/// `[awake]`: the awake test's datagrams and the R3's histogram. Each value's
/// reason is beside it in `bench-host.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AwakeSettings {
    /// Datagrams sent before the R3 starts timing.
    pub(crate) warmup_datagrams: u32,
    /// Datagrams sent while the R3 times.
    pub(crate) datagrams: u32,
    /// The shortest and longest gap between datagrams, ms; each gap is drawn
    /// evenly between them.
    pub(crate) gap_min_ms: u64,
    pub(crate) gap_max_ms: u64,
    /// Each datagram's rate is drawn evenly from ± this, pulses/s.
    pub(crate) top_rate: u16,
    /// The width of each of the R3's histogram bins, µs: 1 to 1,000
    /// (`awake_timer.ino`, `MAX_BIN_US`).
    pub(crate) bin_us: u16,
    /// How many bins: 1 to 250 (`awake_timer.ino`, `MAX_BINS`).
    pub(crate) bins: u16,
    /// The random draws' seed: the same seed sends the same datagrams.
    pub(crate) seed: u64,
    /// Where each run's folder goes, relative to the settings file's folder.
    pub(crate) output_dir: String,
}
