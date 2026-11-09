use crate::camera::frame::Frame;

/// What the reader thread has: the newest frame, how many it has read, and,
/// once the camera has stopped, why.
#[derive(Debug, Default)]
pub(crate) struct LatestState {
    pub(crate) frame: Option<Frame>,
    pub(crate) seq: u64,
    pub(crate) ended: Option<String>,
}
