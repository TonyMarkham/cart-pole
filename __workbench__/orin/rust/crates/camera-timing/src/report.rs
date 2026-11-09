use crate::{
    calibration::Calibration,
    constants::{NANOS_PER_MILLI, PERCENT},
    flash::Flash,
    frame_record::FrameRecord,
    stats::Stats,
};

use std::fmt::Write;

/// `flashes.csv`'s columns, in the order each row writes them: the flash's
/// number and four stamps and its count of lit frames; then the frame that
/// caught it (4 columns); then gap 1's band and middle (3 columns). A row
/// whose flash no frame caught leaves those blank: `NO_FRAME` is the frame's
/// 4 empty fields, `NO_GAP` gap 1's 3 and the line's end.
const FLASHES_HEADER: &str = "n,on_before_ns,on_after_ns,off_before_ns,off_after_ns,lit_frames,\
                              frame_seq,frame_arrived_ns,frame_stamp_ns,frame_level,\
                              gap1_low_ms,gap1_high_ms,gap1_ms\n";
const NO_FRAME: &str = ",,,,";
const NO_GAP: &str = ",,,\n";

/// `frames.csv`'s columns, in the order each row writes them.
const FRAMES_HEADER: &str = "seq,arrived_ns,stamp_ns,level\n";

/// The whole run: the calibration, every flash and every frame of the flash
/// phase, and the camera's exposure, frame period and flash length, ns.
pub(crate) struct Report {
    pub(crate) calibration: Calibration,
    pub(crate) exposure_ns: f64,
    pub(crate) period_ns: f64,
    pub(crate) pulse_ns: f64,
    pub(crate) flashes: Vec<Flash>,
    pub(crate) frames: Vec<FrameRecord>,
}

impl Report {
    /// The summary, for `summary.txt` and the screen.
    pub(crate) fn summary(&self) -> String {
        let mut text = String::new();
        let caught = self
            .flashes
            .iter()
            .filter(|flash| flash.caught_by.is_some())
            .count();
        let several = self
            .flashes
            .iter()
            .filter(|flash| flash.lit_frames > 1)
            .count();
        let gaps: Vec<_> = self
            .flashes
            .iter()
            .filter_map(|flash| flash.gap_one(self.exposure_ns))
            .collect();
        // A flash overlaps an exposure if it starts less than one flash
        // before the exposure or during it: (E + pulse) of each period.
        let expected = PERCENT * (self.exposure_ns + self.pulse_ns) / self.period_ns;
        // Writing to a String never fails.
        let _ = writeln!(
            text,
            "gap 1, the camera exposes a frame -> the frame is read:"
        );
        let _ = writeln!(
            text,
            "  flashes: {}; seen by a frame {caught} ({:.1}%, about {expected:.1}% expected); \
             lit more than one frame {several}",
            self.flashes.len(),
            PERCENT * caught as f64 / self.flashes.len().max(1) as f64
        );
        let mids: Vec<f64> = gaps
            .iter()
            .map(|gap| gap.mid_ns() / NANOS_PER_MILLI)
            .collect();
        if let Some(stats) = Stats::of(&mids) {
            let half = gaps.iter().map(|gap| gap.half_width_ns()).sum::<f64>()
                / gaps.len() as f64
                / NANOS_PER_MILLI;
            let _ = writeln!(
                text,
                "  {} values: mean {:.3} ms, standard deviation {:.3} ms, shortest {:.3} ms, \
                 longest {:.3} ms; each within ±{half:.3} ms",
                stats.count, stats.mean, stats.std_dev, stats.min, stats.max
            );
            for &(percent, value) in &stats.percentiles {
                let _ = writeln!(text, "  {percent}% at most {value:.3} ms");
            }
        }
        let calibration = &self.calibration;
        let _ = writeln!(
            text,
            "LED patch: columns {}..{}, rows {}..{}; dark {:.1}, lit {:.1}, threshold {:.1}",
            calibration.patch.left,
            calibration.patch.right,
            calibration.patch.top,
            calibration.patch.bottom,
            calibration.off_level,
            calibration.on_level,
            calibration.threshold()
        );
        text
    }

    /// `flashes.csv`: every flash's four stamps, the frame that caught it,
    /// and gap 1's band and middle, blank if no one frame caught it.
    pub(crate) fn flashes_csv(&self) -> String {
        let mut text = String::from(FLASHES_HEADER);
        for (n, flash) in self.flashes.iter().enumerate() {
            let _ = write!(
                text,
                "{n},{:.0},{:.0},{:.0},{:.0},{}",
                flash.on_before_ns,
                flash.on_after_ns,
                flash.off_before_ns,
                flash.off_after_ns,
                flash.lit_frames
            );
            match flash.caught_by {
                Some(frame) => {
                    let _ = write!(
                        text,
                        ",{},{:.0},{},{:.1}",
                        frame.seq, frame.arrived_ns, frame.stamp_ns, frame.level
                    );
                }
                None => text.push_str(NO_FRAME),
            }
            match flash.gap_one(self.exposure_ns) {
                Some(gap) => {
                    let _ = writeln!(
                        text,
                        ",{:.3},{:.3},{:.3}",
                        gap.low_ns / NANOS_PER_MILLI,
                        gap.high_ns / NANOS_PER_MILLI,
                        gap.mid_ns() / NANOS_PER_MILLI
                    );
                }
                None => text.push_str(NO_GAP),
            }
        }
        text
    }

    /// `frames.csv`: every frame of the flash phase, with its arrival,
    /// GStreamer's stamp and its patch's level.
    pub(crate) fn frames_csv(&self) -> String {
        let mut text = String::from(FRAMES_HEADER);
        for frame in &self.frames {
            let _ = writeln!(
                text,
                "{},{:.0},{},{:.1}",
                frame.seq, frame.arrived_ns, frame.stamp_ns, frame.level
            );
        }
        text
    }
}
