use crate::{
    protocol::{
        KEY_BIN, KEY_BIN_US, KEY_COUNT, KEY_MAX_TICKS, KEY_MIN_TICKS, KEY_MISSED, KEY_OVER,
        KEY_SPAN_TICKS, KEY_SUM_TICKS, KEY_TICK_NS,
    },
    units::{MICROS_PER_SECOND, NANOS_PER_MICRO, PERCENT},
};

use anyhow::{Context, Result, bail};
use std::fmt::Write;

/// The percentiles the summary gives, in percent.
const PERCENTILES: [f64; 5] = [50.0, 90.0, 99.0, 99.9, 99.99];

/// The R3's report of one run (`awake_timer.ino`, its header), parsed.
pub(crate) struct AwakeReport {
    /// One count of the R3's clock, ns.
    tick_ns: u64,
    /// High stretches timed.
    count: u64,
    /// Edges whose level matched the one before.
    missed: u64,
    /// Stretches longer than the histogram.
    over: u64,
    /// The shortest, longest and total stretch, and the run's length, in
    /// counts of the R3's clock.
    min_ticks: u64,
    max_ticks: u64,
    sum_ticks: u64,
    span_ticks: u64,
    /// Each bin's width, µs.
    bin_us: u64,
    /// (bin, stretches in it), for the bins that hold any.
    bins: Vec<(u64, u64)>,
}

impl AwakeReport {
    pub(crate) fn parse(lines: &[String]) -> Result<Self> {
        let mut report = Self {
            tick_ns: 0,
            count: 0,
            missed: 0,
            over: 0,
            min_ticks: 0,
            max_ticks: 0,
            sum_ticks: 0,
            span_ticks: 0,
            bin_us: 0,
            bins: Vec::new(),
        };
        for line in lines {
            let mut words = line.split_whitespace();
            let (Some(key), Some(value)) = (words.next(), words.next()) else {
                continue;
            };
            let number = |text: &str| -> Result<u64> {
                text.parse()
                    .with_context(|| format!("the R3's report: `{line}`"))
            };
            match key {
                KEY_TICK_NS => report.tick_ns = number(value)?,
                KEY_COUNT => report.count = number(value)?,
                KEY_MISSED => report.missed = number(value)?,
                KEY_OVER => report.over = number(value)?,
                KEY_MIN_TICKS => report.min_ticks = number(value)?,
                KEY_MAX_TICKS => report.max_ticks = number(value)?,
                KEY_SUM_TICKS => report.sum_ticks = number(value)?,
                KEY_SPAN_TICKS => report.span_ticks = number(value)?,
                KEY_BIN_US => report.bin_us = number(value)?,
                KEY_BIN => {
                    let count = words
                        .next()
                        .with_context(|| format!("the R3's report: `{line}`"))?;
                    report.bins.push((number(value)?, number(count)?));
                }
                _ => {}
            }
        }
        if report.tick_ns == 0 || report.bin_us == 0 {
            bail!("the R3's report has no {KEY_TICK_NS} or {KEY_BIN_US}: {lines:?}");
        }
        Ok(report)
    }

    fn micros(&self, ticks: u64) -> f64 {
        ticks as f64 * self.tick_ns as f64 / NANOS_PER_MICRO
    }

    /// The stretch below which `percent` of them fall, µs: the upper edge of
    /// the bin that holds it, so "at most this". None if it lies among the
    /// stretches longer than the histogram.
    fn percentile(&self, percent: f64) -> Option<u64> {
        let wanted = (self.count as f64 * percent / PERCENT).ceil() as u64;
        let mut seen = 0;
        for &(bin, count) in &self.bins {
            seen += count;
            if seen >= wanted {
                return Some((bin + 1) * self.bin_us);
            }
        }
        None
    }

    /// The summary, for `summary.txt` and the screen.
    pub(crate) fn summary(&self) -> String {
        let mut text = String::new();
        let span_s = self.micros(self.span_ticks) / MICROS_PER_SECOND;
        // Writing to a String never fails.
        let _ = writeln!(text, "awake stretches timed: {}", self.count);
        let _ = writeln!(
            text,
            "over {:.1} s: {:.1} wakes per second; awake {:.2}% of the time",
            span_s,
            if span_s > 0.0 {
                self.count as f64 / span_s
            } else {
                0.0
            },
            if self.span_ticks > 0 {
                PERCENT * self.sum_ticks as f64 / self.span_ticks as f64
            } else {
                0.0
            }
        );
        let _ = writeln!(
            text,
            "shortest {:.1} us, mean {:.1} us, longest {:.1} us",
            self.micros(self.min_ticks),
            if self.count > 0 {
                self.micros(self.sum_ticks) / self.count as f64
            } else {
                0.0
            },
            self.micros(self.max_ticks)
        );
        for percent in PERCENTILES {
            match self.percentile(percent) {
                Some(us) => {
                    let _ = writeln!(text, "{percent}% of stretches at most {us} us");
                }
                None => {
                    let _ = writeln!(text, "{percent}% of stretches: beyond the histogram");
                }
            }
        }
        let _ = writeln!(
            text,
            "beyond the histogram: {}; edges missed: {}",
            self.over, self.missed
        );
        text
    }
}
