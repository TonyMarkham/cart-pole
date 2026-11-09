use crate::constants::BRAIN_IDLE;

use serde::Deserialize;

/// The webserver's answer to STOP: the brain's status after idle, or its own
/// words when the brain gave none (`state` `"unknown"`), with whether the
/// command 0 went out to the board. It answers this shape whether or not
/// either step worked, so the page reads it whatever the HTTP status.
#[derive(Debug, Deserialize)]
pub(crate) struct StopAnswer {
    #[serde(default)]
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) message: String,
    pub(crate) zeros_sent: bool,
    #[serde(default)]
    pub(crate) zeros_error: Option<String>,
}

impl StopAnswer {
    /// What STOP did, in words for the page. The zeros come first: they are
    /// what stops the cart.
    pub(crate) fn words(&self) -> String {
        let zeros = if self.zeros_sent {
            "The board was sent the command 0.".to_string()
        } else {
            format!(
                "The command 0 could not be sent: {}. Use the e-stop.",
                self.zeros_error.as_deref().unwrap_or("no reason given")
            )
        };
        let brain = if self.state == BRAIN_IDLE {
            "The brain is idle.".to_string()
        } else {
            format!("The brain did not say it is idle: {}", self.message)
        };
        format!("STOP: {zeros} {brain}")
    }
}
