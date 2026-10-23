use serde::Deserialize;

/// Synthetic Orin → orchestrator, exactly one text message per frame, in
/// order: `{"id": 7}`, or `{"id": 7, "error": "..."}` when the frame broke
/// the 728×544 JPEG contract (O3), which fails the run. Fields added later
/// (the policy's `command`) are ignored until the loop uses them.
#[derive(Deserialize, Debug)]
pub struct OrinReply {
    pub id: u32,
    pub error: Option<String>,
}
