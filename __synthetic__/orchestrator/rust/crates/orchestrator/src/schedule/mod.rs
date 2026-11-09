pub(crate) mod step_times;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    orin::orin_reply::POOL_ENTRIES,
    schedule::step_times::StepTimes,
    timing::{Timing, bands::Bands, step_picks::StepPicks},
};

/// The pre-warm's steps, −5 to −1: the pool's 6 entries less step 0's own
/// (synthetic-loop-dia.md, *The initial pool*).
pub const PRE_WARM_STEPS: i64 = POOL_ENTRIES as i64 - 1;

/// Spare steps after the episode's last, since generating more than is
/// needed costs nothing (user, 2026-11-08: steps 500 to 514 at 500).
pub const SPARE_STEPS: i64 = 15;

/// The episode's schedule (synthetic-loop-dia.md, *The schedule*): steps
/// −5 to the episode's last + 15, made at the reset and never changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
    steps: Vec<StepTimes>,
    timestep_ms: f64,
}

/// `ms` to the nearest whole timestep, halfway going later (towards +∞,
/// negative times included): with 2 ms, 69 → 35 (70 ms), −33 → −16 (−32 ms).
pub fn nearest_timestep(ms: f64, timestep_ms: f64) -> i64 {
    // A schedule's times are a few seconds at most, far inside an i64.
    (ms / timestep_ms + 0.5).floor() as i64
}

impl Schedule {
    /// The episode's schedule from its seed: 5 + `max_episode_steps` + 15
    /// steps, each step's five picks in turn from step −5 up.
    pub fn make(seed: u64, bands: &Bands, max_episode_steps: u32, timestep_ms: f64) -> Schedule {
        let count = PRE_WARM_STEPS + i64::from(max_episode_steps) + SPARE_STEPS;
        let mut timing = Timing::new(seed);
        let picks: Vec<StepPicks> = (0..count).map(|_| timing.step_picks(bands)).collect();
        Schedule::from_picks(&picks, timestep_ms)
    }

    /// The schedule from the picks of steps −5, −4, … in order. Exact times
    /// in ms from step 0's capture, as running totals: picture ready(0) =
    /// stretch 1(0), so capture(0) = 0; picture ready(k + 1) = send(k) +
    /// the wait(k), worked backwards for the pre-warm's steps; capture =
    /// picture ready − stretch 1, send = picture ready + stretch 2, motor
    /// change = send + stretch 3. Each entry is rounded as it is written.
    /// `picks` must reach step 0.
    pub fn from_picks(picks: &[StepPicks], timestep_ms: f64) -> Schedule {
        let zero = PRE_WARM_STEPS as usize;
        let mut ready = vec![0.0; picks.len()];
        ready[zero] = picks[zero].frame_ready;
        for i in zero + 1..picks.len() {
            ready[i] = ready[i - 1] + picks[i - 1].to_uno + picks[i - 1].next_frame_ready;
        }
        for i in (0..zero).rev() {
            ready[i] = ready[i + 1] - picks[i].to_uno - picks[i].next_frame_ready;
        }
        let round = |ms| nearest_timestep(ms, timestep_ms);
        let steps = picks
            .iter()
            .zip(&ready)
            .map(|(pick, &ready)| {
                let send = ready + pick.to_uno;
                StepTimes {
                    capture: round(ready - pick.frame_ready),
                    picture_ready: round(ready),
                    send: round(send),
                    motor_change: (!pick.lost).then(|| round(send + pick.change_motor)),
                }
            })
            .collect();
        Schedule { steps, timestep_ms }
    }

    /// Step k's timings, k from −5; None past either end.
    pub fn step(&self, k: i64) -> Option<&StepTimes> {
        usize::try_from(k + PRE_WARM_STEPS)
            .ok()
            .and_then(|i| self.steps.get(i))
    }

    /// The schedule's last step: the episode's last + 15.
    pub fn last_step(&self) -> i64 {
        self.steps.len() as i64 - PRE_WARM_STEPS - 1
    }

    pub fn timestep_ms(&self) -> f64 {
        self.timestep_ms
    }

    /// The Orin's pre-warm gaps, ms (synthetic-loop-dia.md, *The initial
    /// pool*): d₁ = 0 − C₋₁, d₂ = C₋₁ − C₋₂, …, d₅ = C₋₄ − C₋₅ from the
    /// rounded captures, and d₆ = d₅. Pushing step 0 drops the 6th entry at
    /// once, so d₆ never reaches the policy and any value would do.
    pub fn pool_gaps(&self) -> [f64; POOL_ENTRIES] {
        let capture = |k: i64| self.steps[(k + PRE_WARM_STEPS) as usize].capture;
        let mut gaps = [0.0; POOL_ENTRIES];
        for (j, gap) in gaps.iter_mut().take(POOL_ENTRIES - 1).enumerate() {
            let newer = -(j as i64);
            *gap = (capture(newer) - capture(newer - 1)) as f64 * self.timestep_ms;
        }
        gaps[POOL_ENTRIES - 1] = gaps[POOL_ENTRIES - 2];
        gaps
    }
}
