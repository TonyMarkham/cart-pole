use crate::run::run_state::RunState;

/// What the loop thread and the requests share: the state, why the last run
/// ended or Run was refused, the run's folder, how many frames the run has
/// acted on, and the request for the loop to stop.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) state: RunState,
    pub(crate) message: String,
    pub(crate) run_dir: Option<String>,
    pub(crate) frames: u64,
    /// Set by `idle`; the loop looks at it before each frame.
    pub(crate) stop: bool,
}

impl Default for Shared {
    fn default() -> Self {
        Shared {
            state: RunState::Idle,
            message: String::new(),
            run_dir: None,
            frames: 0,
            stop: false,
        }
    }
}
