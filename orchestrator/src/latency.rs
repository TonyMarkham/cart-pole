/// The Orin's latency band, ms (synthetic-loop-dia.md: RND(8, 15), the
/// detector's measured inference time over 250 test images).
const LATENCY_MS: (f64, f64) = (8.0, 15.0);

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
}

impl Latency {
    pub fn new(seed: u64) -> Latency {
        Latency { state: seed }
    }

    /// The next 64-bit SplitMix64 output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The next latency, ms, in [8, 15): the output's top 53 bits as
    /// u ∈ [0, 1), then 8 + 7u.
    pub fn next_ms(&mut self) -> f64 {
        // A u64 shifted right by 11 has at most 53 bits, so the cast to
        // f64 is exact.
        let u = (self.next_u64() >> 11) as f64 / TWO_POW_53;
        LATENCY_MS.0 + (LATENCY_MS.1 - LATENCY_MS.0) * u
    }
}
