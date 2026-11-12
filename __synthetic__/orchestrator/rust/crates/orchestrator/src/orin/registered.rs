use serde::Deserialize;

/// Synthetic Orin → orchestrator, a message with only a type: once, before
/// any pose, `{"type": "registered"}` (__documentation__/synthetic-loop-dia.md, 8) — the
/// Orin has connected to the player's frame socket, so the observation side
/// is ready and the loop may start; and after each reset, `{"type": "reset"}`
/// (*Reset in place*, R7) — the pool is pre-warmed again.
#[derive(Deserialize, Debug)]
pub struct Registered {
    #[serde(rename = "type")]
    pub kind: String,
}
