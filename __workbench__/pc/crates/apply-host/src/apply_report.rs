use crate::{
    apply_result::ApplyResult, protocol::R3_TICKS_PER_US, result_kind::ResultKind, stats::Stats,
};

use std::fmt::Write;

/// The whole run: for each command sent while the R3 timed, its rate and the
/// R3's result, or None if no line came about it.
pub(crate) struct ApplyReport {
    pub(crate) results: Vec<(u16, Option<ApplyResult>)>,
}

impl ApplyReport {
    /// The applied commands' delays, µs.
    fn delays_us(&self) -> Vec<f64> {
        self.results
            .iter()
            .filter_map(|(_, result)| *result)
            .filter(|result| result.kind == ResultKind::Applied)
            .map(|result| result.ticks as f64 / R3_TICKS_PER_US)
            .collect()
    }

    fn count(&self, kind: ResultKind) -> usize {
        self.results
            .iter()
            .filter(|(_, result)| result.is_some_and(|result| result.kind == kind))
            .count()
    }

    /// The summary, for `summary.txt` and the screen.
    pub(crate) fn summary(&self) -> String {
        let mut text = String::new();
        let missing = self
            .results
            .iter()
            .filter(|(_, result)| result.is_none())
            .count();
        // Writing to a String never fails.
        let _ = writeln!(
            text,
            "gap 8, the R4 has a command whole -> its new rate on STEP:"
        );
        let _ = writeln!(
            text,
            "  commands: {} sent while timing; applied {}, changed before the mark {}, never changed {}, no line from the R3 {}",
            self.results.len(),
            self.count(ResultKind::Applied),
            self.count(ResultKind::Early),
            self.count(ResultKind::None),
            missing
        );
        if let Some(stats) = Stats::of(&self.delays_us()) {
            let _ = writeln!(
                text,
                "  {} delays: mean {:.0} us, standard deviation {:.0} us, shortest {:.0} us, longest {:.0} us",
                stats.count, stats.mean, stats.std_dev, stats.min, stats.max
            );
            for &(percent, value) in &stats.percentiles {
                let _ = writeln!(text, "  {percent}% at most {value:.0} us");
            }
        }
        text
    }

    /// `results.csv`: every command, its rate, and the R3's result, blank
    /// where no line came.
    pub(crate) fn results_csv(&self) -> String {
        let mut text = String::from("n,rate,kind,delay_us,old_period_us,new_period_us\n");
        for (n, (rate, result)) in self.results.iter().enumerate() {
            match result {
                Some(result) => {
                    let _ = writeln!(
                        text,
                        "{n},{rate},{:?},{:.3},{:.3},{:.3}",
                        result.kind,
                        result.ticks as f64 / R3_TICKS_PER_US,
                        result.old_period as f64 / R3_TICKS_PER_US,
                        result.new_period as f64 / R3_TICKS_PER_US
                    );
                }
                None => {
                    let _ = writeln!(text, "{n},{rate},,,,");
                }
            }
        }
        text
    }

    /// `histogram.csv`: the applied delays in bins of `bin_us`, from 0 to the
    /// longest's bin.
    pub(crate) fn histogram_csv(&self, bin_us: u32) -> String {
        let mut text = String::from("from_us,to_us,count\n");
        let width = f64::from(bin_us.max(1));
        let delays = self.delays_us();
        let Some(longest) = delays.iter().copied().reduce(f64::max) else {
            return text;
        };
        let mut counts = vec![0u64; (longest / width) as usize + 1];
        for delay in delays {
            if let Some(count) = counts.get_mut((delay / width) as usize) {
                *count += 1;
            }
        }
        for (index, count) in counts.iter().enumerate() {
            let from = index as f64 * width;
            let _ = writeln!(text, "{from:.0},{:.0},{count}", from + width);
        }
        text
    }
}
