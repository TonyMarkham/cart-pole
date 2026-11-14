use crate::{camera::camera_transform::CameraTransform, orin::joint_qpos::JointQpos};

use serde::Serialize;

/// Orchestrator → synthetic Orin (__documentation__/synthetic-loop-dia.md, 11), one text
/// message per observation: `{"id": 7, "time": 0.14, "joints":
/// [{"name": "02-Cart.Slider.joint", "qpos": 0.05}, {"name":
/// "04-Pole.Hinge.joint", "qpos": 3.14}]}`. `time` is the capture time in
/// simulated seconds; the Orin forwards `{"id", "joints"}` to Unity as
/// the trigger (12) and answers with exactly one reply (14). When the run
/// has a camera pose, `"cameraTransform"` rides along and the Orin forwards
/// it too; without one the key is left out.
#[derive(Serialize, Debug)]
pub struct PoseMessage {
    pub id: u32,
    pub time: f64,
    pub joints: Vec<JointQpos>,
    #[serde(rename = "cameraTransform", skip_serializing_if = "Option::is_none")]
    pub camera_transform: Option<CameraTransform>,
}
