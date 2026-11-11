use crate::constants::{CLASSES, DETECTION_FLOATS};

use serde::Serialize;

/// One line of `frames.jsonl` (`__documentation__/real-loop-dia.md`, *The log*): `boxes`, per
/// class in class order the detector's centre, size and confidence; `points`,
/// Pivot's and End's (x, z) in metres, or (−1, −1); `inputs`, the 36 numbers
/// sent to the policy; `stamp`, the frame's GStreamer timestamp, ns; `a` and
/// `c`, the policy's answer and the command sent; `arrived` and `sent`, ms
/// since the Run press on the Orin's monotonic clock.
#[derive(Debug, Serialize)]
pub(crate) struct FrameLine<'a> {
    pub(crate) boxes: [[f64; DETECTION_FLOATS]; CLASSES],
    pub(crate) points: [[f64; 2]; 2],
    pub(crate) inputs: &'a [f64],
    pub(crate) stamp: u64,
    pub(crate) a: f64,
    pub(crate) c: i32,
    pub(crate) arrived: f64,
    pub(crate) sent: f64,
}
