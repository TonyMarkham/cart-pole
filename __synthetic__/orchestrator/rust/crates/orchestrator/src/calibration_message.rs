use serde::{Deserialize, Serialize};

use crate::joint_qpos::JointQpos;

/// Orchestrator → calibrating Orin (synthetic-loop-dia.md, Calibration, C6):
/// measure this pose, whose Pivot and End truly are at these `(x, z)`, in metres.
#[derive(Serialize, Debug)]
pub struct MeasureMessage<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub pose: &'a str,
    pub joints: Vec<JointQpos>,
    pub pivot: [f64; 2],
    pub end: [f64; 2],
}

/// Orchestrator → calibrating Orin (C8): fit the matrix to what was measured.
#[derive(Serialize, Debug)]
pub struct FitMessage {
    #[serde(rename = "type")]
    pub kind: &'static str,
}

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

/// Calibrating Orin → orchestrator (C9): whether every point lay within the
/// limit, in which case the matrix is written; the worst point, in pixels; and
/// the lens number.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FittedReply {
    #[serde(rename = "type")]
    pub kind: String,
    pub ok: bool,
    pub worst_px: f64,
    pub lens_k: f64,
}
