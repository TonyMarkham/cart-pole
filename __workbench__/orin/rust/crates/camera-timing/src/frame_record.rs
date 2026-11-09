/// One frame of the flash phase, for `frames.csv`: its number, when it
/// arrived (ns since the run started, the Orin's monotonic clock), GStreamer's
/// stamp for it, and its patch's level.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameRecord {
    pub(crate) seq: u64,
    pub(crate) arrived_ns: f64,
    pub(crate) stamp_ns: u64,
    pub(crate) level: f64,
}
