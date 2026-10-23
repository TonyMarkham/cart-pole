use serde::Serialize;

/// Orchestrator → synthetic Orin, one WebSocket text message each
/// (orin-plan.md, Messages):
/// `{"type": "reset", "frame_interval": 0.02}` at episode start (T in
/// seconds of simulated time), and `{"type": "frame", "id": 7, "time":
/// 0.14}` before each frame, followed by one binary message, the JPEG.
#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrinMessage {
    Reset { frame_interval: f64 },
    Frame { id: u32, time: f64 },
}
