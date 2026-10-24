use serde::Deserialize;

/// Synthetic Orin → orchestrator, exactly one text message per pose, in
/// order: `{"id": 7}`, or `{"id": 7, "error": "..."}` when Unity's frame
/// broke the 728×544 JPEG contract (O3), which fails the run. Fields added
/// later (the pool, `failed`; synthetic-loop-dia.md, 14) are ignored
/// until the loop uses them.
#[derive(Deserialize, Debug)]
pub struct OrinReply {
    pub id: u32,
    pub error: Option<String>,
}
