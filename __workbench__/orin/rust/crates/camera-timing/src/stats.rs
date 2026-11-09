use crate::constants::PERCENT;

/// The percentiles reported, in percent.
pub(crate) const PERCENTILES: [f64; 4] = [50.0, 90.0, 99.0, 99.9];

/// The spread of a set of values, in their own unit: the count, the mean,
/// the sample standard deviation (with n − 1, the usual estimate of the
/// spread a larger run would show), the extremes, and the percentiles, each
/// the nearest-rank value: the smallest value with at least that share of
/// the set at or below it.
#[derive(Clone, Debug)]
pub(crate) struct Stats {
    pub(crate) count: usize,
    pub(crate) mean: f64,
    pub(crate) std_dev: f64,
    pub(crate) min: f64,
    pub(crate) max: f64,
    /// (percent, value), one per `PERCENTILES`.
    pub(crate) percentiles: Vec<(f64, f64)>,
}

impl Stats {
    /// None for an empty set.
    pub(crate) fn of(values: &[f64]) -> Option<Self> {
        let count = values.len();
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let (&min, &max) = (sorted.first()?, sorted.last()?);
        let n = count as f64;
        let mean = sorted.iter().sum::<f64>() / n;
        let std_dev = if count > 1 {
            (sorted
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (n - 1.0))
                .sqrt()
        } else {
            0.0
        };
        let percentiles = PERCENTILES
            .iter()
            .map(|&percent| {
                let rank = (n * percent / PERCENT).ceil().max(1.0) as usize;
                (percent, sorted[rank.min(count) - 1])
            })
            .collect();
        Some(Self {
            count,
            mean,
            std_dev,
            min,
            max,
            percentiles,
        })
    }
}
