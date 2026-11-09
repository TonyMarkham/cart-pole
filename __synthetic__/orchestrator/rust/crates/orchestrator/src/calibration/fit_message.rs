use serde::Serialize;

/// Orchestrator → calibrating Orin (C8): fit the matrix to what was measured.
#[derive(Serialize, Debug)]
pub struct FitMessage {
    #[serde(rename = "type")]
    pub kind: &'static str,
}
