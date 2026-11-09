use serde::Deserialize;

/// `[worker]`: the brain's worker and the matrix, the brain's own values
/// (its `[brain]` table), each explained in `stage-timing.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkerSettings {
    /// The folder the worker runs in, and the command that starts it there. A
    /// leading `~` is the home folder.
    pub(crate) worker_dir: String,
    pub(crate) worker: Vec<String>,
    /// How long the worker gets to load and say it is ready, ms.
    pub(crate) worker_start_timeout_ms: u64,
    /// The fitted matrix. A leading `~` is the home folder.
    pub(crate) homography: String,
}
