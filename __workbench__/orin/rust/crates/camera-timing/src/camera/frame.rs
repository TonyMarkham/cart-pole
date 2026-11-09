use std::{sync::Arc, time::Instant};

/// One frame as the camera gave it: 728 × 544 greyscale, one byte per pixel,
/// top row first, with GStreamer's stamp. `seq` counts the frames since the
/// camera opened. `arrived` is when the reader had the whole frame, on the
/// Orin's monotonic clock: the same moment the brain's log calls `arrived`
/// (the brain's `camera/frame.rs`).
#[derive(Clone, Debug)]
pub(crate) struct Frame {
    pub(crate) seq: u64,
    /// GStreamer's `do-timestamp` stamp for the buffer, ns.
    pub(crate) stamp_ns: u64,
    pub(crate) arrived: Instant,
    pub(crate) pixels: Arc<[u8]>,
}
