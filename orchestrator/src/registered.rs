use serde::{Deserialize, Serialize};

/// Synthetic Orin → orchestrator, a message with only a type: once, before
/// any pose, `{"type": "registered"}` (synthetic-loop-dia.md, 8) — Unity
/// has connected to the Orin's frame socket, so the observation side is
/// ready and the loop may start; and after each reset, `{"type": "reset"}`
/// (*Reset in place*, R7) — the pool is pre-warmed again.
#[derive(Deserialize, Debug)]
pub struct Registered {
    #[serde(rename = "type")]
    pub kind: String,
}

/// Orchestrator → synthetic Orin (*Reset in place*, R5): `{"type":
/// "reset", "seed": S}` — pre-warm the pool afresh, its gaps from S.
#[derive(Serialize, Debug)]
pub struct ResetMessage {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub seed: u64,
}
