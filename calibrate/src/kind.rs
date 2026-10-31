use crate::{
    profile::{Profile, Segment},
    settings::{HOLD_MS, STEADY_MS, TRIAL_DT_MS, TRIAL_TOP},
};

/// The kinds of trial, the jumps a policy makes (jump-limit-experiment.md,
/// section 5, step 3). `top` is `trial_top`, Δt is `trial_dt_ms`, and a
/// change rate j makes a jump of j × Δt pulses/s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 0 to +j × Δt.
    FromRest,
    /// top ÷ 2 to top ÷ 2 + j × Δt.
    AtSpeed,
    /// −c to +c, with 2c = j × Δt.
    Reversal,
    /// From 0, jumps of j × Δt every Δt until top.
    Staircase,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::FromRest, Kind::AtSpeed, Kind::Reversal, Kind::Staircase];

    pub const fn name(self) -> &'static str {
        match self {
            Kind::FromRest => "from rest",
            Kind::AtSpeed => "at speed",
            Kind::Reversal => "reversal",
            Kind::Staircase => "staircase",
        }
    }

    /// The largest j the kind can make with every rate inside ±top: a
    /// jump from rest or a staircase step is at most top; one from
    /// top ÷ 2 at most top ÷ 2; a reversal, −top to +top, at most 2 × top.
    /// Above it the brief calls the kind's limit "at least" this (section
    /// 5, step 5).
    pub const fn j_max(self) -> u32 {
        let jump = match self {
            Kind::FromRest | Kind::Staircase => TRIAL_TOP,
            Kind::AtSpeed => TRIAL_TOP / 2,
            Kind::Reversal => TRIAL_TOP * 2,
        };
        (jump as u64 * 1_000 / TRIAL_DT_MS as u64) as u32
    }

    /// The rates either side of the kind's jump at change rate `j`, for
    /// the log: (c_a, c_b). A staircase's are its first step.
    pub fn rates(self, j: u32) -> (i32, i32) {
        let jump = jump(j);
        match self {
            Kind::FromRest | Kind::Staircase => (0, jump),
            Kind::AtSpeed => (TRIAL_TOP as i32 / 2, TRIAL_TOP as i32 / 2 + jump),
            Kind::Reversal => (-(jump / 2), jump / 2),
        }
    }

    /// The trial's whole rate plan at change rate `j`, from rest to rest:
    /// reach c_a gently and hold it `steady_ms`, jump to c_b and hold it
    /// `hold_ms` (a staircase climbs instead), then stop gently. None if
    /// `j` is above `j_max` or the plan does not fit.
    pub fn plan(self, j: u32) -> Option<Profile> {
        if j > self.j_max() {
            return None;
        }
        let (c_a, c_b) = self.rates(j);
        let mut plan = Profile::new();
        let mut ok = true;
        if c_a != 0 {
            ok &= plan.push(Segment::Ramp { from: 0, to: c_a });
        }
        ok &= plan.push(Segment::Hold { rate: c_a, ms: STEADY_MS });
        let last = match self {
            Kind::Staircase => {
                let top = TRIAL_TOP as i32;
                let mut rate = 0;
                while rate < top && ok {
                    rate = (rate + jump(j)).min(top);
                    ok &= plan.push(Segment::Hold { rate, ms: TRIAL_DT_MS });
                }
                top
            }
            _ => c_b,
        };
        ok &= plan.push(Segment::Hold { rate: last, ms: HOLD_MS });
        ok &= plan.push(Segment::Ramp { from: last, to: 0 });
        ok.then_some(plan)
    }
}

/// j × Δt: the jump a change rate `j` makes in one gap, pulses/s.
fn jump(j: u32) -> i32 {
    (u64::from(j) * u64::from(TRIAL_DT_MS) / 1_000) as i32
}
