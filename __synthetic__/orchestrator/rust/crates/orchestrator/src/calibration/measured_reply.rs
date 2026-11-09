use serde::Deserialize;

/// Calibrating Orin → orchestrator (C7): the pose is measured, from `images`
/// images that had both markers, out of `renders`.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct MeasuredReply {
    #[serde(rename = "type")]
    pub kind: String,
    pub pose: String,
    pub images: u32,
    pub renders: u32,
}
