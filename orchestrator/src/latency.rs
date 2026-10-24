use anyhow::{Result, bail};

/// 2^53: a 53-bit integer divided by it is exactly representable in an
/// f64 and lies in [0, 1).
const TWO_POW_53: f64 = 9_007_199_254_740_992.0;

/// The seeded latency draws (orchestrator-plan.md, Configuration): one
/// draw per capture, from SplitMix64 (Steele, Lea and Flood, 2014), so
/// the same seed gives the same latencies on every machine. Each step
/// adds the golden-ratio constant to the state and scrambles it; the
/// test outputs for seed 0 are in tests.rs.
pub struct Latency {
    state: u64,
    /// The band, ms: RND(min, max) (synthetic-loop-dia.md), the trainer's
    /// rnd_delay_min and rnd_delay_max.
    min_ms: f64,
    max_ms: f64,
}

impl Latency {
    /// Fails unless 0 ≤ min ≤ max, both finite.
    pub fn new(seed: u64, min_ms: f64, max_ms: f64) -> Result<Latency> {
        if !(min_ms.is_finite() && max_ms.is_finite() && 0.0 <= min_ms && min_ms <= max_ms) {
            bail!("the latency band is 0 ≤ --rnd-delay-min ≤ --rnd-delay-max ms, finite; got {min_ms} and {max_ms}");
        }
        Ok(Latency { state: seed, min_ms, max_ms })
    }

    /// The next 64-bit SplitMix64 output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The next latency, ms, in [min, max): the output's top 53 bits as
    /// u ∈ [0, 1), then min + (max − min)·u (at 8–15: 8 + 7u).
    pub fn next_ms(&mut self) -> f64 {
        // A u64 shifted right by 11 has at most 53 bits, so the cast to
        // f64 is exact.
        let u = (self.next_u64() >> 11) as f64 / TWO_POW_53;
        self.min_ms + (self.max_ms - self.min_ms) * u
    }
}
