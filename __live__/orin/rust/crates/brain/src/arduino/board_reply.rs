use crate::constants::{REPLY_FAULT, REPLY_STATUS_CONFLICT, REPLY_UNCALIBRATED};

use serde::Deserialize;

/// A reply from the board, in its one answer format: `status`, `result`,
/// `message`, `pin` and `pulses` (`real-loop-dia.md`, *The board's states*,
/// the replies). A command gets one only in FAULT and UNCALIBRATED, and
/// either ends the run.
#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct BoardReply {
    pub(crate) status: u64,
    pub(crate) result: String,
    #[serde(default)]
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) pin: Option<String>,
    #[serde(default)]
    pub(crate) pulses: Option<u64>,
}

impl BoardReply {
    /// The datagram as a reply, or None when it is not one.
    pub(crate) fn parse(datagram: &[u8]) -> Option<BoardReply> {
        serde_json::from_slice(datagram).ok()
    }

    /// Why this reply ends the run, in words for `/run`, or None when it
    /// doesn't: the FAULT STATE reply, and the UNCALIBRATED reply to a command
    /// sent before a calibration.
    pub(crate) fn ends_run(&self) -> Option<String> {
        if self.status != REPLY_STATUS_CONFLICT {
            return None;
        }
        match self.result.as_str() {
            REPLY_FAULT => Some(format!(
                "the board is in FAULT{}{}: reset it, then calibrate",
                self.pin
                    .as_deref()
                    .map_or(String::new(), |pin| format!(", switch {pin}")),
                self.pulses
                    .map_or(String::new(), |pulses| format!(", at {pulses} pulses")),
            )),
            REPLY_UNCALIBRATED => Some("the board is not calibrated: calibrate first".to_string()),
            _ => None,
        }
    }
}
