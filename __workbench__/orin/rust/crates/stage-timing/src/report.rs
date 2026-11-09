use crate::{
    constants::{CLASSES, NANOS_PER_MILLI},
    stage_record::{GAP_NAMES, StageRecord},
    stats::Stats,
};

use std::fmt::Write;

/// `frames.csv`'s columns, in the order each row writes them: the frame, its
/// six moments, what the loop made of it, then gaps 2 to 6 and their total.
const FRAMES_HEADER: &str = "seq,stamp_ns,arrived_ns,frame_sent_ns,boxes_ns,inputs_ns,answered_ns,\
                             sent_ns,found,a,c,gap2_ms,gap3_ms,gap4_ms,gap5_ms,gap6_ms,\
                             gaps2_6_ms\n";

/// `summary.txt`'s words: its first line follows the count of frames; the
/// two lines after the five gaps name what they measure; the last precedes
/// the count of frames with every marker found; and a set with no values is
/// said to have none.
const SAY_FRAMES: &str = "frames, the brain's loop on the Orin's monotonic clock:";
const SAY_TOTAL: &str = "gaps 2-6, the frame arrives -> its command is sent";
const SAY_CADENCE: &str = "gap 9, cadence, one command sent -> the next";
const SAY_ALL_FOUND: &str = "frames with all four markers found:";
const SAY_NO_VALUES: &str = "no values";

/// The whole run: every pass of the loop, in order.
pub(crate) struct Report {
    pub(crate) records: Vec<StageRecord>,
}

impl Report {
    /// The summary, for `summary.txt` and the screen: each gap's spread, the
    /// five together, and the cadence, the time from one command sent to the
    /// next (gap 9).
    pub(crate) fn summary(&self) -> String {
        let mut text = String::new();
        // Writing to a String never fails.
        let _ = writeln!(text, "{} {SAY_FRAMES}", self.records.len());
        for (index, name) in GAP_NAMES.iter().enumerate() {
            let values: Vec<f64> = self
                .records
                .iter()
                .map(|record| record.gaps_ms()[index])
                .collect();
            write_stats(&mut text, name, &values);
        }
        let totals: Vec<f64> = self.records.iter().map(StageRecord::total_ms).collect();
        write_stats(&mut text, SAY_TOTAL, &totals);
        let cadence: Vec<f64> = self
            .records
            .windows(2)
            .map(|pair| (pair[1].sent_ns - pair[0].sent_ns) / NANOS_PER_MILLI)
            .collect();
        write_stats(&mut text, SAY_CADENCE, &cadence);
        let all_found = self
            .records
            .iter()
            .filter(|record| record.found == CLASSES)
            .count();
        let _ = writeln!(
            text,
            "{SAY_ALL_FOUND} {all_found} of {}",
            self.records.len()
        );
        text
    }

    /// `frames.csv`: every pass, its moments and its gaps.
    pub(crate) fn frames_csv(&self) -> String {
        let mut text = String::from(FRAMES_HEADER);
        for record in &self.records {
            let _ = write!(
                text,
                "{},{},{:.0},{:.0},{:.0},{:.0},{:.0},{:.0},{},{:.6},{}",
                record.seq,
                record.stamp_ns,
                record.arrived_ns,
                record.frame_sent_ns,
                record.boxes_ns,
                record.inputs_ns,
                record.answered_ns,
                record.sent_ns,
                record.found,
                record.a,
                record.c
            );
            for gap in record.gaps_ms() {
                let _ = write!(text, ",{gap:.4}");
            }
            let _ = writeln!(text, ",{:.4}", record.total_ms());
        }
        text
    }
}

/// One set's line, `name: count, mean, standard deviation, extremes`, then
/// its percentiles on the next, every value in ms.
fn write_stats(text: &mut String, name: &str, values: &[f64]) {
    let Some(stats) = Stats::of(values) else {
        let _ = writeln!(text, "{name}: {SAY_NO_VALUES}");
        return;
    };
    let _ = writeln!(
        text,
        "{name}: {} values, mean {:.3} ms, standard deviation {:.3} ms, shortest {:.3} ms, \
         longest {:.3} ms",
        stats.count, stats.mean, stats.std_dev, stats.min, stats.max
    );
    let percentiles: Vec<String> = stats
        .percentiles
        .iter()
        .map(|(percent, value)| format!("{percent}% at most {value:.3} ms"))
        .collect();
    let _ = writeln!(text, "  {}", percentiles.join(", "));
}
