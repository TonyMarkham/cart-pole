use crate::board_answer::BoardAnswer;

use serde::Deserialize;

/// The webserver's `/run/status`: the brain's status (`__documentation__/real-loop-dia.md`,
/// *Contracts*, the brain's requests) and, while the brain is idle, the
/// board's answer to `state`, or why there is none.
#[derive(Debug, Deserialize)]
pub(crate) struct RunStatus {
    /// `"run"` or `"idle"`.
    pub(crate) state: String,
    /// Why the last run ended, or why Run was refused.
    #[serde(default)]
    pub(crate) message: String,
    /// The run's log folder, from the first Run on.
    #[serde(default)]
    pub(crate) run_dir: Option<String>,
    /// How many frames the run has acted on.
    #[serde(default)]
    pub(crate) frames: u64,
    /// Null during a run, when the board is not asked, or when it gave no
    /// answer.
    #[serde(default)]
    pub(crate) board: Option<BoardAnswer>,
    /// Why `board` is null when the board was asked.
    #[serde(default)]
    pub(crate) board_error: Option<String>,
}
