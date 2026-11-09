use serde::Deserialize;

/// `[wifi]`: the WiFi test. Each value's reason is beside it in
/// `wifi-host.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WifiSettings {
    /// Clock syncs before the first datagram, and after the last.
    pub(crate) warmup_syncs: u32,
    pub(crate) closing_syncs: u32,
    /// Datagrams sent, one per cycle.
    pub(crate) cycles: u32,
    /// The shortest and longest gap after each cycle, ms; each is drawn
    /// evenly between them.
    pub(crate) gap_min_ms: u64,
    pub(crate) gap_max_ms: u64,
    /// How long a sync's answer may take, ms, before the sync is lost.
    pub(crate) sync_timeout_ms: u64,
    /// How long a datagram's report may take, ms, before it is lost.
    pub(crate) ping_timeout_ms: u64,
    /// Each datagram's rate is drawn from 0 to this, pulses/s, with a random
    /// sign.
    pub(crate) top_rate: u16,
    /// How many runs of consecutive syncs the fit splits them into.
    pub(crate) fit_windows: usize,
    /// The share of each window's syncs the fit keeps, the shortest round
    /// trips: more than 0, at most 1.
    pub(crate) keep_fraction: f64,
    /// The histogram's bin width, µs.
    pub(crate) histogram_bin_us: u32,
    /// The random draws' seed.
    pub(crate) seed: u64,
    /// Where each run's folder goes, relative to the settings file's folder.
    pub(crate) output_dir: String,
}
