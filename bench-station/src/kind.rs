use crate::{
    profile::{MAX_SEGMENTS, Segment},
    settings::Experiment,
};

/// The kinds of trial, the jumps a policy makes (jump-limit-experiment.md,
/// section 5, step 3), as `calibrate/src/kind.rs` defines them. `top` is
/// `trial_top`, Δt is `trial_dt_ms`, and a change rate j makes a jump of
/// j × Δt pulses/s.
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
    pub fn j_max(self, settings: &Experiment) -> u32 {
        let top = u64::from(settings.trial_top);
        let jump = match self {
            Kind::FromRest | Kind::Staircase => top,
            Kind::AtSpeed => top / 2,
            Kind::Reversal => top * 2,
        };
        (jump * 1_000 / u64::from(settings.trial_dt_ms.max(1))).min(u64::from(u32::MAX)) as u32
    }

    /// Every j the kind tries if none slips: from `j_start`, raised by
    /// `j_growth_percent` each step and capped at `j_max`, the same rule
    /// the firmware runs (`calibrate/src/experiment.rs`). A slip ends the
    /// kind early, so this is the most steps it can take.
    pub fn j_steps(self, settings: &Experiment) -> Vec<u32> {
        let most = self.j_max(settings);
        let mut steps = Vec::new();
        let mut j = settings.j_start;
        while j <= most {
            steps.push(j);
            if j == most {
                break;
            }
            let raised = (u64::from(j) * u64::from(100 + settings.j_growth_percent) / 100).min(u64::from(most)) as u32;
            j = raised.max(j + 1);
        }
        steps
    }

    /// The rates either side of the kind's jump at change rate `j`:
    /// (c_a, c_b). A staircase's are its first step.
    pub fn rates(self, j: u32, settings: &Experiment) -> (i32, i32) {
        let jump = jump(j, settings);
        let half_top = (settings.trial_top / 2) as i32;
        match self {
            Kind::FromRest | Kind::Staircase => (0, jump),
            Kind::AtSpeed => (half_top, half_top + jump),
            Kind::Reversal => (-(jump / 2), jump / 2),
        }
    }

    /// The trial's whole rate plan at change rate `j`, from rest to rest:
    /// reach c_a gently and hold it `steady_ms`, jump to c_b and hold it
    /// `hold_ms` (a staircase climbs instead), then stop gently. None if
    /// `j` is above `j_max` or the plan has more pieces than the Arduino
    /// holds.
    pub fn plan(self, j: u32, settings: &Experiment) -> Option<Vec<Segment>> {
        if j > self.j_max(settings) {
            return None;
        }
        let (c_a, c_b) = self.rates(j, settings);
        let mut plan = Vec::new();
        if c_a != 0 {
            plan.push(Segment::Ramp { from: 0, to: c_a });
        }
        plan.push(Segment::Hold { rate: c_a, ms: settings.steady_ms });
        let last = match self {
            Kind::Staircase => {
                let top = settings.trial_top as i32;
                let step = jump(j, settings).max(1);
                let mut rate = 0;
                while rate < top && plan.len() <= MAX_SEGMENTS {
                    rate = (rate + step).min(top);
                    plan.push(Segment::Hold { rate, ms: settings.trial_dt_ms });
                }
                top
            }
            _ => c_b,
        };
        plan.push(Segment::Hold { rate: last, ms: settings.hold_ms });
        plan.push(Segment::Ramp { from: last, to: 0 });
        (plan.len() <= MAX_SEGMENTS).then_some(plan)
    }
}

/// j × Δt: the jump a change rate `j` makes in one gap, pulses/s.
fn jump(j: u32, settings: &Experiment) -> i32 {
    (u64::from(j) * u64::from(settings.trial_dt_ms) / 1_000) as i32
}
