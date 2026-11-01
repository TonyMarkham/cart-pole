/// Where a slow move ends if no switch trips first.
#[derive(Clone, Copy, Debug)]
pub enum Limit {
    /// After this many pulses: the move to the centre.
    Pulses(u32),
    /// After this many ms: a seek giving up.
    Time(u32),
}
