use serde::Serialize;

use crate::joint_qpos::JointQpos;

/// Orchestrator → synthetic Orin (synthetic-loop-dia.md, 11), one text
/// message per observation: `{"id": 7, "time": 0.14, "joints":
/// [{"name": "02-Cart.Slider.joint", "qpos": 0.05}, {"name":
/// "04-Pole.Hinge.joint", "qpos": 3.14}]}`. `time` is the capture time in
/// simulated seconds; the Orin forwards `{"id", "joints"}` to Unity as
/// the trigger (12) and answers with exactly one reply (14).
#[derive(Serialize, Debug)]
pub struct PoseMessage {
    pub id: u32,
    pub time: f64,
    pub joints: Vec<JointQpos>,
}
