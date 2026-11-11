use crate::run::{run_state::RunState, shared::Shared};

use serde::Serialize;

/// The brain's answer to every request (`__documentation__/real-loop-dia.md`, *Contracts*, the
/// brain's requests): its state, the words for `/run` to show, the run's
/// folder and how many frames it has acted on.
#[derive(Debug, Serialize)]
pub(crate) struct StatusAnswer {
    pub(crate) state: RunState,
    pub(crate) message: String,
    pub(crate) run_dir: Option<String>,
    pub(crate) frames: u64,
}

impl StatusAnswer {
    pub(crate) fn of(shared: &Shared) -> StatusAnswer {
        StatusAnswer {
            state: shared.state,
            message: shared.message.clone(),
            run_dir: shared.run_dir.clone(),
            frames: shared.frames,
        }
    }
}
