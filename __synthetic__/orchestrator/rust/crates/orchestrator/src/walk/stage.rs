/// Where the walk is within the current timestep (__documentation__/synthetic-loop-dia.md,
/// *Events on a shared timestep*): each timestep goes MoveOn, then its
/// motor changes, Send, Tick, Capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Capture,
    MoveOn,
    Send,
    Tick,
}
