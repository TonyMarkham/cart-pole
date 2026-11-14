use crate::{camera::camera_transform::CameraTransform, orin::joint_qpos::JointQpos};

use serde::Serialize;

/// Orchestrator → calibrating Orin (__documentation__/synthetic-loop-dia.md, Calibration, C6):
/// measure this pose, whose Pivot and End truly are at these `(x, z)`, in metres.
/// `"cameraTransform"` as in the pose message, left out without one.
#[derive(Serialize, Debug)]
pub struct MeasureMessage<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub pose: &'a str,
    pub joints: Vec<JointQpos>,
    pub pivot: [f64; 2],
    pub end: [f64; 2],
    #[serde(rename = "cameraTransform", skip_serializing_if = "Option::is_none")]
    pub camera_transform: Option<CameraTransform>,
}
