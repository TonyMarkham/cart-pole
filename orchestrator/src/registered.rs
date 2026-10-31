use serde::{Deserialize, Serialize};

use crate::orin_reply::POOL_ENTRIES;

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
/// "reset", "gaps": [d₁, …, d₆]}` — pre-warm the pool afresh at these gaps,
/// ms, the orchestrator's draws: the Orin holds no timing of its own.
#[derive(Serialize, Debug)]
pub struct ResetMessage {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub gaps: [f64; POOL_ENTRIES],
}
