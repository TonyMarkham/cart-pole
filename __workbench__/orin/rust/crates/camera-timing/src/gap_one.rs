/// A midpoint is the mean of a pair: their sum over this.
const PAIR: f64 = 2.0;

/// Gap 1 for one flash a frame caught: from that frame's exposure starting to
/// the frame being read, ns, as the band it must lie in.
///
/// The rule: an exposure that starts at `s` and lasts `E` shows the flash only
/// if the two overlap, so `s` lies between `on − E` and `off`, with `on` the
/// earliest the LED can have lit (stamped just before the write that lit it)
/// and `off` the latest it can have gone out (stamped just after the write
/// that put it out). Gap 1 is `arrived − s`, so it lies between
/// `arrived − off` and `arrived − on + E`. The band's width, `E` plus the
/// flash's length, is the measurement's resolution; its middle is the value
/// reported.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GapOne {
    pub(crate) low_ns: f64,
    pub(crate) high_ns: f64,
}

impl GapOne {
    pub(crate) fn of(arrived_ns: f64, on_ns: f64, off_ns: f64, exposure_ns: f64) -> GapOne {
        GapOne {
            low_ns: arrived_ns - off_ns,
            high_ns: arrived_ns - on_ns + exposure_ns,
        }
    }

    pub(crate) fn mid_ns(&self) -> f64 {
        (self.low_ns + self.high_ns) / PAIR
    }

    /// Half the band: how far the true value can be from `mid_ns`.
    pub(crate) fn half_width_ns(&self) -> f64 {
        (self.high_ns - self.low_ns) / PAIR
    }
}
