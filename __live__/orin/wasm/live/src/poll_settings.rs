use serde::Deserialize;

/// The part of the webserver's `/calibrate/state` this page uses.
#[derive(Debug, Deserialize)]
pub(crate) struct PollSettings {
    pub(crate) poll_ms: u32,
}
