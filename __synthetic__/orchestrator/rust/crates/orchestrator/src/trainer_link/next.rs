use serde::Deserialize;

/// What the trainer wants after an instance's pool (18): go on with the
/// command, reset the instance in place for its next episode
/// (synthetic-loop-dia.md, *Reset in place*), or stop the run.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Next {
    Continue,
    Reset,
    Stop,
}
