use crate::orin::orin_reply::POOL_ENTRIES;

use serde::Serialize;

/// Orchestrator → synthetic Orin (*Reset in place*, R5): `{"type":
/// "reset", "gaps": [d₁, …, d₆]}` — pre-warm the pool afresh at these gaps,
/// ms, from the new episode's schedule (*The initial pool*): the Orin holds
/// no timing of its own.
#[derive(Serialize, Debug)]
pub struct ResetMessage {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub gaps: [f64; POOL_ENTRIES],
}
