use crate::{frame_record::FrameRecord, gap_one::GapOne};

/// One flash: the four stamps around the two GPIO writes (ns since the run
/// started), the first frame after it whose patch was lit, and how many
/// frames in its window were lit, which should be 0 or 1.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Flash {
    pub(crate) on_before_ns: f64,
    pub(crate) on_after_ns: f64,
    pub(crate) off_before_ns: f64,
    pub(crate) off_after_ns: f64,
    pub(crate) caught_by: Option<FrameRecord>,
    pub(crate) lit_frames: u32,
}

impl Flash {
    /// Gap 1 for this flash, if one frame, and only one, caught it.
    pub(crate) fn gap_one(&self, exposure_ns: f64) -> Option<GapOne> {
        let frame = self.caught_by.filter(|_| self.lit_frames == 1)?;
        Some(GapOne::of(
            frame.arrived_ns,
            self.on_before_ns,
            self.off_after_ns,
            exposure_ns,
        ))
    }
}
