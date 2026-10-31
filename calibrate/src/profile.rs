use crate::settings::{ACCEL, SEEK_SPEED};

/// The most pieces one plan holds. A staircase is the longest: one step
/// per j × Δt up to the top, and the build script holds j_start high
/// enough that the steps fit.
pub const MAX_SEGMENTS: usize = 48;

/// One piece of a trial's pulse-rate plan. Rates are pulses/s, + in Plus.
#[derive(Clone, Copy, Debug)]
pub enum Segment {
    /// `rate` held for `ms`; a rate of 0 sends nothing.
    Hold { rate: i32, ms: u32 },
    /// From `from` to `to` at `accel`, the gentle change the brief asks
    /// for to reach a starting rate and to stop (jump-limit-experiment.md,
    /// section 4, *Stop gently*). It never runs below seek_speed except
    /// at an end that is 0: from rest it starts at seek_speed, and into
    /// rest it stops from it, the seek's own jump.
    Ramp { from: i32, to: i32 },
}

impl Segment {
    /// The piece's length, µs.
    fn duration_us(self) -> u64 {
        match self {
            Segment::Hold { ms, .. } => u64::from(ms) * 1_000,
            Segment::Ramp { from, to } => u64::from(from.abs_diff(to)) * 1_000_000 / u64::from(ACCEL),
        }
    }

    /// The rate `t_us` into the piece.
    fn rate_at(self, t_us: u64) -> i32 {
        match self {
            Segment::Hold { rate, .. } => rate,
            Segment::Ramp { from, to } => {
                let change = (u64::from(ACCEL) * t_us / 1_000_000).min(u64::from(from.abs_diff(to))) as i64;
                let (from, to) = (i64::from(from), i64::from(to));
                let rate = if to > from { from + change } else { from - change };
                let floor = i64::from(SEEK_SPEED).min(from.abs().max(to.abs()));
                if rate != 0 && rate.abs() < floor { (rate.signum() * floor) as i32 } else { rate as i32 }
            }
        }
    }
}

/// A trial's whole rate plan: its pieces back to back. A change of rate
/// between two pieces is a jump, made at one instant.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    segments: [Segment; MAX_SEGMENTS],
    len: usize,
}

impl Profile {
    pub const fn new() -> Self {
        Self {
            segments: [Segment::Hold { rate: 0, ms: 0 }; MAX_SEGMENTS],
            len: 0,
        }
    }

    /// Adds a piece; false if the plan is full.
    pub fn push(&mut self, segment: Segment) -> bool {
        if self.len == MAX_SEGMENTS {
            return false;
        }
        self.segments[self.len] = segment;
        self.len += 1;
        true
    }

    /// The plan's length, µs.
    pub fn duration_us(&self) -> u64 {
        self.segments[..self.len].iter().map(|segment| segment.duration_us()).sum()
    }

    /// The rate `t_us` from the start; 0 after the end.
    pub fn rate_at(&self, t_us: u64) -> i32 {
        let mut start = 0;
        for &segment in &self.segments[..self.len] {
            let end = start + segment.duration_us();
            if t_us < end {
                return segment.rate_at(t_us - start);
            }
            start = end;
        }
        0
    }

    /// The cart's lowest and highest place during the plan, pulses from
    /// where it starts: the rate summed over 1 ms steps.
    pub fn excursion(&self) -> (i64, i64) {
        let (mut low, mut high, mut milli_pulses) = (0, 0, 0i64);
        let mut t_us = 0;
        let end = self.duration_us();
        while t_us < end {
            milli_pulses += i64::from(self.rate_at(t_us));
            let place = milli_pulses / 1_000;
            low = low.min(place);
            high = high.max(place);
            t_us += 1_000;
        }
        (low, high)
    }
}
