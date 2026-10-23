use serde::Serialize;

use crate::joint_qpos::JointQpos;

/// Orchestrator → player, one WebSocket text message per state:
/// `{"id": 7, "joints": [{"name": "02-Cart.Slider.joint", "qpos": 0.05}, ...]}`.
/// The player answers each with exactly one binary message (the frame's
/// JPEG), in order.
#[derive(Serialize, Debug)]
pub struct StateMessage {
    pub id: u32,
    pub joints: Vec<JointQpos>,
}
