use crate::trainer_link::request::Request;

use serde::Deserialize;

/// The trainer's first stdin line, `{"request": "run"}`
/// (synthetic-loop-dia.md, Contracts, the run's values).
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct RequestMessage {
    #[allow(dead_code)]
    pub request: Request,
}
