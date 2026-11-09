use crate::{
    protocol::R4_NOMINAL_HZ,
    units::{NANOS_PER_SECOND, PAIR},
};

/// One clock sync over USB, NTP's four stamps: `t0` the PC's just before it
/// wrote the request, `t1` the R4's when it had the request whole, `t2` the
/// R4's just before its answer's first byte, `t3` the PC's when it had the
/// answer whole. The PC's are ns since the run started; the R4's are its
/// clock's raw 32-bit counts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SyncExchange {
    pub(crate) number: u32,
    pub(crate) t0_ns: f64,
    pub(crate) t1: u32,
    pub(crate) t2: u32,
    pub(crate) t3_ns: f64,
}

impl SyncExchange {
    /// The round trip, ns: the PC's whole wait less the R4's time between its
    /// two stamps, which is converted at the R4's design rate (a few µs, so
    /// the rate's tolerance changes it by nanoseconds). The two stamps are
    /// microseconds apart, so their 32-bit difference never wraps.
    pub(crate) fn round_trip_ns(&self) -> f64 {
        let held = f64::from(self.t2.wrapping_sub(self.t1)) / R4_NOMINAL_HZ * NANOS_PER_SECOND;
        (self.t3_ns - self.t0_ns) - held
    }

    /// The PC's midpoint, ns: if the line takes the same time each way, the
    /// R4's midpoint happened then.
    pub(crate) fn pc_mid_ns(&self) -> f64 {
        (self.t0_ns + self.t3_ns) / PAIR
    }
}
