use anyhow::{Result, bail};

/// The R4's clock in the PC's time: a straight line from the R4's wide count
/// to the PC's ns, `pc = offset + rate × count`, fitted by least squares
/// through the kept syncs' midpoints. `rate` is the PC's ns per R4 count, so
/// 1 / `rate` is the R4's true frequency as the PC's clock sees it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClockFit {
    pub(crate) offset_ns: f64,
    pub(crate) rate_ns_per_count: f64,
}

impl ClockFit {
    /// The line through `points`, each (R4 wide count, PC ns). At least two
    /// points with different counts are needed for a line.
    pub(crate) fn fit(points: &[(f64, f64)]) -> Result<Self> {
        if points.len() < 2 {
            bail!("a clock fit needs at least two syncs, not {}", points.len());
        }
        let n = points.len() as f64;
        let mean_x = points.iter().map(|&(x, _)| x).sum::<f64>() / n;
        let mean_y = points.iter().map(|&(_, y)| y).sum::<f64>() / n;
        let sxx: f64 = points.iter().map(|&(x, _)| (x - mean_x).powi(2)).sum();
        let sxy: f64 = points
            .iter()
            .map(|&(x, y)| (x - mean_x) * (y - mean_y))
            .sum();
        if sxx == 0.0 {
            bail!("the kept syncs all have the same R4 count");
        }
        let rate = sxy / sxx;
        Ok(Self {
            offset_ns: mean_y - rate * mean_x,
            rate_ns_per_count: rate,
        })
    }

    /// The PC's time, ns, when the R4's clock read `count`.
    pub(crate) fn pc_ns(&self, count: f64) -> f64 {
        self.offset_ns + self.rate_ns_per_count * count
    }
}
