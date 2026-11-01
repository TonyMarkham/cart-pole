use crate::travel::Travel;

use serde::Deserialize;

/// The webserver's `/calibrate/state`.
#[derive(Debug, Deserialize)]
pub(crate) struct ServerState {
    pub(crate) last_stop: Option<Travel>,
    pub(crate) span: Option<u32>,
    pub(crate) poll_ms: u32,
}
