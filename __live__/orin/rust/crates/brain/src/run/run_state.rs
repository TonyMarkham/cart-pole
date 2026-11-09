use serde::Serialize;

/// The brain's two states (`real-loop-dia.md`, *The run and idle states*). A
/// page on the webserver asks it to change between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RunState {
    Idle,
    Run,
}
