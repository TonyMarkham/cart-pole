use crate::orin::joint_qpos::JointQpos;

use serde::Serialize;

/// Orchestrator → calibrating Orin (__documentation__/synthetic-loop-dia.md, Calibration, C6):
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
