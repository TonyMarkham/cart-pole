pub(crate) mod band;
pub(crate) mod bands;
pub(crate) mod step_picks;

// ---------------------------------------------------------------------------------------------- //

use crate::timing::{band::Band, bands::Bands, step_picks::StepPicks};

/// 2^53: a 53-bit integer divided by it is exactly representable in an
/// f64 and lies in [0, 1).
pub(crate) const TWO_POW_53: f64 = 9_007_199_254_740_992.0;

/// The seeded random timings (orchestrator-plan.md, Configuration): every
/// random time in the loop comes from this one generator, SplitMix64
/// (Steele, Lea and Flood, 2014), so the same seed gives the same times on
/// every machine. Each pick adds the golden-ratio constant to the state and
/// scrambles it; the test outputs for seed 0 are in tests.rs. Seeded with
/// the episode's seed at its reset, it makes the whole schedule and nothing
/// after.
pub struct Timing {
    state: u64,
}

impl Timing {
    pub fn new(seed: u64) -> Timing {
        Timing { state: seed }
    }

    /// The next 64-bit SplitMix64 output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// u ∈ [0, 1): the next output's top 53 bits ÷ 2^53. A u64 shifted
    /// right by 11 has at most 53 bits, so the cast to f64 is exact.
    fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / TWO_POW_53
    }

    /// RND(min, max): min + (max − min)·u, in [min, max).
    pub fn pick(&mut self, band: Band) -> f64 {
        band.min + (band.max - band.min) * self.next_unit()
    }

    /// A whole number from 1 to `n`, 1 + ⌊n·u⌋, coming out 1: lost, about
    /// once in n (exactly when u < 1 ÷ n).
    pub fn pick_lost(&mut self, n: u32) -> bool {
        (f64::from(n) * self.next_unit()).floor() < 1.0
    }

    /// One step's five picks, in order: stretch 1, stretch 2, stretch 3,
    /// the wait, the drop.
    pub fn step_picks(&mut self, bands: &Bands) -> StepPicks {
        StepPicks {
            frame_ready: self.pick(bands.frame_ready),
            to_uno: self.pick(bands.to_uno),
            change_motor: self.pick(bands.change_motor),
            next_frame_ready: self.pick(bands.next_frame_ready),
            lost: self.pick_lost(bands.drop_one_message_in),
        }
    }
}
