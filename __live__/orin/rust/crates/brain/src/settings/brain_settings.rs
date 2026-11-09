use serde::Deserialize;

/// `[brain]`: the brain's own values. `settings.toml` explains each where it
/// is set.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BrainSettings {
    /// Where the brain answers the webserver's requests, the Orin's own
    /// address.
    pub(crate) address: String,
    /// The folder the worker runs in, the Orin's Python, and the command that
    /// starts it there. A leading `~` is the home folder.
    pub(crate) worker_dir: String,
    pub(crate) worker: Vec<String>,
    /// How long the worker gets to load the engine and the policy and say it is
    /// ready, ms.
    pub(crate) worker_start_timeout_ms: u64,
    /// The wait after each command before the next frame is taken, ms (user,
    /// 2026-11-02).
    pub(crate) delay_ms: u64,
    /// The gap between the pre-warmed pool's entries, ms.
    pub(crate) pre_warm_gap_ms: f64,
    /// How long to wait for a frame before the camera counts as gone, ms.
    pub(crate) frame_timeout_ms: u64,
    /// Where each run's folder is made. A leading `~` is the home folder.
    pub(crate) runs_dir: String,
    /// The fitted matrix, `calibrate_service.py`'s `matrix_path`. A leading `~`
    /// is the home folder.
    pub(crate) homography: String,
    /// Where `calibrate_service.py` listens, asked whether it has the camera,
    /// and how long to wait for its answer, ms.
    pub(crate) service: String,
    pub(crate) service_timeout_ms: u64,
}
