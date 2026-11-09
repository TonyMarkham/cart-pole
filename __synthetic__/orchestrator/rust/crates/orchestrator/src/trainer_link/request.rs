use serde::Deserialize;

/// What the trainer's first stdin line asks for: the run's values.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Request {
    Run,
}
