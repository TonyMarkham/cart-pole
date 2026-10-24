use serde::Deserialize;

/// Synthetic Orin → orchestrator, once, before any pose
/// (synthetic-loop-dia.md, 8): `{"type": "registered"}` — Unity has
/// connected to the Orin's frame socket, so the observation side is
/// ready and the loop may start.
#[derive(Deserialize, Debug)]
pub struct Registered {
    #[serde(rename = "type")]
    pub kind: String,
}
