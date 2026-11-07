use crate::constants::{STATUS_BAD_GATEWAY, STATUS_OK};

use serde::Deserialize;

/// How a move ended: the board's five keys, or the webserver's, which uses the
/// same ones when the board gave no answer.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) result: String,
    pub(crate) message: String,
    pub(crate) pin: Option<String>,
    pub(crate) pulses: u32,
}

impl Answer {
    /// A move whose answer never reached the page. The cart may well have moved.
    pub(crate) fn lost(message: String) -> Self {
        Self {
            status: STATUS_BAD_GATEWAY,
            result: "no_answer".to_string(),
            message,
            pin: None,
            pulses: 0,
        }
    }

    pub(crate) fn succeeded(&self) -> bool {
        self.status == STATUS_OK
    }
}
