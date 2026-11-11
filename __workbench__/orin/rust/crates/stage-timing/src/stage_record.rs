use crate::constants::NANOS_PER_MILLI;

/// The gaps measured, in the order a frame passes through them
/// (`__documentation__/real-timings.md`, gaps 2 to 6), as `summary.txt` names them.
pub(crate) const GAP_NAMES: [&str; 5] = [
    "gap 2, hand-off to the worker",
    "gap 3, detection",
    "gap 4, the brain between stages",
    "gap 5, policy inference",
    "gap 6, send",
];

/// One pass of the loop: its frame, and the six moments it passed, each in
/// ns since the run started on the Orin's monotonic clock. Each gap is the
/// difference of two neighbours, so the five always add up to `sent` less
/// `arrived`, the brain's share (gaps 2–6).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StageRecord {
    pub(crate) seq: u64,
    /// GStreamer's stamp for the frame, ns.
    pub(crate) stamp_ns: u64,
    /// The reader had the whole frame: the brain's `arrived`.
    pub(crate) arrived_ns: f64,
    /// The frame's last byte was in the worker's pipe.
    pub(crate) frame_sent_ns: f64,
    /// The boxes were back from the worker.
    pub(crate) boxes_ns: f64,
    /// The 36 inputs were built: metres, the pool, the history.
    pub(crate) inputs_ns: f64,
    /// a was back from the worker.
    pub(crate) answered_ns: f64,
    /// The datagram was handed to the system: the brain's `sent`.
    pub(crate) sent_ns: f64,
    /// How many of the four markers were found.
    pub(crate) found: usize,
    pub(crate) a: f64,
    pub(crate) c: i32,
}

impl StageRecord {
    /// Gaps 2 to 6, ms, in `GAP_NAMES`' order.
    pub(crate) fn gaps_ms(&self) -> [f64; 5] {
        let moments = [
            self.arrived_ns,
            self.frame_sent_ns,
            self.boxes_ns,
            self.inputs_ns,
            self.answered_ns,
            self.sent_ns,
        ];
        let mut gaps = [0f64; 5];
        for (gap, pair) in gaps.iter_mut().zip(moments.windows(2)) {
            *gap = (pair[1] - pair[0]) / NANOS_PER_MILLI;
        }
        gaps
    }

    /// Gaps 2–6 together, ms: `sent` less `arrived`, the brain's log's own
    /// measure.
    pub(crate) fn total_ms(&self) -> f64 {
        (self.sent_ns - self.arrived_ns) / NANOS_PER_MILLI
    }
}
