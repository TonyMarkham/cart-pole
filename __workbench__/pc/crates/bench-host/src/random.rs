/// SplitMix64's constants (Steele, Lea and Flood, "Fast splittable
/// pseudorandom number generators", OOPSLA 2014; Vigna's reference
/// `splitmix64.c`): the state's step, the golden ratio's fraction × 2⁶⁴, then
/// the output mixer's two multipliers and three shifts.
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
const MIX_MULTIPLIER_1: u64 = 0xBF58_476D_1CE4_E5B9;
const MIX_MULTIPLIER_2: u64 = 0x94D0_49BB_1331_11EB;
const MIX_SHIFT_1: u32 = 30;
const MIX_SHIFT_2: u32 = 27;
const MIX_SHIFT_3: u32 = 31;

/// The bench's random draws: SplitMix64, the same sequence for the same
/// seed, so a run can be repeated exactly. Written out here rather than taken
/// from a crate: it is eight lines, and its sequence then never changes with
/// a dependency's version.
pub(crate) struct Random {
    state: u64,
}

impl Random {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> MIX_SHIFT_1)).wrapping_mul(MIX_MULTIPLIER_1);
        z = (z ^ (z >> MIX_SHIFT_2)).wrapping_mul(MIX_MULTIPLIER_2);
        z ^ (z >> MIX_SHIFT_3)
    }

    /// A whole number from `low` to `high`, both included, each about equally
    /// likely. Taking the remainder favours the low values by at most
    /// span ÷ 2⁶⁴: under one part in 10¹⁴ for the bench's widest span, the
    /// 89,201 rates of ±44,600.
    pub(crate) fn between(&mut self, low: i64, high: i64) -> i64 {
        let span = high.abs_diff(low) + 1;
        let offset = self.next_u64() % span;
        low.saturating_add_unsigned(offset)
    }
}
