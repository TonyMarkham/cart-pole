/// One band of a random timing, ms: RND(min, max) (synthetic-loop-dia.md,
/// *Timing*, *The random timings*).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub min: f64,
    pub max: f64,
}
