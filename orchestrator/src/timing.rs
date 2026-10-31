use anyhow::{Result, bail};

use crate::orin_reply::POOL_ENTRIES;

/// 2^53: a 53-bit integer divided by it is exactly representable in an
/// f64 and lies in [0, 1).
const TWO_POW_53: f64 = 9_007_199_254_740_992.0;

/// The seeded timing draws (orchestrator-plan.md, Configuration): every
/// random time in the loop comes from this one generator, SplitMix64
/// (Steele, Lea and Flood, 2014), so the same seed gives the same times on
/// every machine. Each step adds the golden-ratio constant to the state and
/// scrambles it; the test outputs for seed 0 are in tests.rs. One generator
/// serves both bands: two seeded alike would draw the same number each
/// time, and tie the latency to the command rate.
pub struct Timing {
    state: u64,
    /// The Orin's latency, ms: RND(min, max) (synthetic-loop-dia.md), the
    /// trainer's rnd_delay_min and rnd_delay_max.
    latency_ms: (f64, f64),
    /// The command rate, Hz: RND(min, max), the gym file's command_rate_min
    /// and command_rate_max.
    rate_hz: (f64, f64),
}

impl Timing {
    /// Fails unless 0 ≤ the latency's min ≤ its max and 0 < the rate's min
    /// ≤ its max, all finite.
    pub fn new(seed: u64, latency_ms: (f64, f64), rate_hz: (f64, f64)) -> Result<Timing> {
        let (low, high) = latency_ms;
        if !(low.is_finite() && high.is_finite() && 0.0 <= low && low <= high) {
            bail!("the latency band is 0 ≤ --rnd-delay-min ≤ --rnd-delay-max ms, finite; got {low} and {high}");
        }
        let (low, high) = rate_hz;
        if !(low.is_finite() && high.is_finite() && 0.0 < low && low <= high) {
            bail!("the command rate's band is 0 < --command-rate-min ≤ --command-rate-max Hz, finite; got {low} and {high}");
        }
        Ok(Timing { state: seed, latency_ms, rate_hz })
    }

    /// The same bands from a new seed (R4).
    pub fn reseeded(&self, seed: u64) -> Timing {
        Timing { state: seed, latency_ms: self.latency_ms, rate_hz: self.rate_hz }
    }

    /// The next 64-bit SplitMix64 output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The next value of `band`, in [min, max): the output's top 53 bits as
    /// u ∈ [0, 1), then min + (max − min)·u.
    fn next_in(&mut self, band: (f64, f64)) -> f64 {
        // A u64 shifted right by 11 has at most 53 bits, so the cast to
        // f64 is exact.
        let u = (self.next_u64() >> 11) as f64 / TWO_POW_53;
        band.0 + (band.1 - band.0) * u
    }

    /// The next latency, ms (at 8–15: 8 + 7u).
    pub fn next_latency_ms(&mut self) -> f64 {
        self.next_in(self.latency_ms)
    }

    /// The next gap to a capture, ms: 1000 ÷ the next command rate (at
    /// 20–30 Hz: 1000 ÷ (20 + 10u), 33.3 to 50 ms).
    pub fn next_gap_ms(&mut self) -> f64 {
        1000.0 / self.next_in(self.rate_hz)
    }

    /// The new pool's six pre-warm gaps, ms, d₁ first: an episode's first
    /// draws (synthetic-loop-dia.md, The initial pool).
    pub fn next_pool_gaps(&mut self) -> [f64; POOL_ENTRIES] {
        std::array::from_fn(|_| self.next_gap_ms())
    }

    /// The shortest gap must be at least the longest latency + one timestep,
    /// or a command might not act for one step before the next capture: the
    /// arrival step is at most the latency + one timestep after the capture,
    /// and the next capture later than the gap after it.
    pub fn check_commands_act(&self, timestep_s: f64) -> Result<()> {
        let shortest_ms = 1000.0 / self.rate_hz.1;
        let needed_ms = self.latency_ms.1 + timestep_s * 1000.0;
        if shortest_ms < needed_ms {
            bail!(
                "--command-rate-max {} Hz leaves {shortest_ms} ms between captures, less than --rnd-delay-max {} ms + the {timestep_s} s timestep: a command might never act",
                self.rate_hz.1,
                self.latency_ms.1
            );
        }
        Ok(())
    }
}
