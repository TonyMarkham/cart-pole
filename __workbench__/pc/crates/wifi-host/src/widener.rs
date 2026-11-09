use crate::{
    protocol::{R4_NOMINAL_HZ, R4_WRAP},
    units::NANOS_PER_SECOND,
};

/// Widens the R4's 32-bit stamps, which wrap every 89.5 s, into one count
/// from the run's first stamp. Each stamp comes with the PC's time near it,
/// so the PC predicts the wide count at the design rate and picks the one
/// wrap of the raw stamp nearest that prediction. The prediction is off by
/// the clock's tolerance over the run, around a second at most for a long
/// run, against the 44.7 s (half a wrap) a wrong pick would need.
pub(crate) struct Widener {
    /// The first stamp, the zero of the wide count, and the PC's time then.
    first_raw: f64,
    first_pc_ns: f64,
}

impl Widener {
    pub(crate) fn new(first_raw: u32, first_pc_ns: f64) -> Self {
        Self {
            first_raw: f64::from(first_raw),
            first_pc_ns,
        }
    }

    /// `raw`, taken near the PC's `pc_ns`, as counts since the first stamp.
    pub(crate) fn widen(&self, raw: u32, pc_ns: f64) -> f64 {
        let predicted = (pc_ns - self.first_pc_ns) * R4_NOMINAL_HZ / NANOS_PER_SECOND;
        let offset = f64::from(raw) - self.first_raw;
        let wraps = ((predicted - offset) / R4_WRAP).round();
        offset + wraps * R4_WRAP
    }
}
