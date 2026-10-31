//! A trial's pulse-rate plan, as `calibrate/src/profile.rs` defines it.
//! The Pi builds the plan and sends it in a `TRIAL` line; the Arduino
//! plays it. The Pi also works out how far the plan carries the cart, so
//! a trial that would run into an end is never sent.

/// The most pieces one plan may hold: the Arduino keeps the plan in a
/// fixed array of this size (`calibrate/src/profile.rs`, MAX_SEGMENTS).
pub const MAX_SEGMENTS: usize = 48;

/// One piece of a plan. Rates are pulses/s, + in Plus.
#[derive(Clone, Copy, Debug)]
pub enum Segment {
    /// `rate` held for `ms`; a rate of 0 sends nothing.
    Hold { rate: i32, ms: u32 },
    /// From `from` to `to` at the firmware's acceleration, never below
    /// the seek rate except at an end that is 0.
    Ramp { from: i32, to: i32 },
}

/// The firmware's ramp: its acceleration and the slowest rate a ramp
/// runs at (bench-station.toml [rig]).
#[derive(Clone, Copy, Debug)]
pub struct Ramping {
    pub accel: u32,
    pub seek_speed: u32,
}

impl Segment {
    /// The piece in a `TRIAL` line: `H:<rate>:<ms>` or `R:<from>:<to>`.
    pub fn token(self) -> String {
        match self {
            Segment::Hold { rate, ms } => format!("H:{rate}:{ms}"),
            Segment::Ramp { from, to } => format!("R:{from}:{to}"),
        }
    }

    /// The piece's length, µs.
    fn duration_us(self, ramping: Ramping) -> u64 {
        match self {
            Segment::Hold { ms, .. } => u64::from(ms) * 1_000,
            Segment::Ramp { from, to } => u64::from(from.abs_diff(to)) * 1_000_000 / u64::from(ramping.accel.max(1)),
        }
    }

    /// The rate `t_us` into the piece.
    fn rate_at(self, t_us: u64, ramping: Ramping) -> i32 {
        match self {
            Segment::Hold { rate, .. } => rate,
            Segment::Ramp { from, to } => {
                let change = (u64::from(ramping.accel) * t_us / 1_000_000).min(u64::from(from.abs_diff(to))) as i64;
                let (from, to) = (i64::from(from), i64::from(to));
                let rate = if to > from { from + change } else { from - change };
                let floor = i64::from(ramping.seek_speed).min(from.abs().max(to.abs()));
                if rate != 0 && rate.abs() < floor { (rate.signum() * floor) as i32 } else { rate as i32 }
            }
        }
    }
}

/// The rate `t_us` from the plan's start; 0 after its end.
fn rate_at(plan: &[Segment], t_us: u64, ramping: Ramping) -> i32 {
    let mut start = 0;
    for &segment in plan {
        let end = start + segment.duration_us(ramping);
        if t_us < end {
            return segment.rate_at(t_us - start, ramping);
        }
        start = end;
    }
    0
}

/// The cart's lowest and highest place during the plan, pulses from where
/// it starts: the rate summed over 1 ms steps.
pub fn excursion(plan: &[Segment], ramping: Ramping) -> (i64, i64) {
    let (mut low, mut high, mut milli_pulses) = (0, 0, 0i64);
    let end: u64 = plan.iter().map(|segment| segment.duration_us(ramping)).sum();
    let mut t_us = 0;
    while t_us < end {
        milli_pulses += i64::from(rate_at(plan, t_us, ramping));
        let place = milli_pulses / 1_000;
        low = low.min(place);
        high = high.max(place);
        t_us += 1_000;
    }
    (low, high)
}
