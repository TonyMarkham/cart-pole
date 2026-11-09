use crate::signals::Dm542Signals;

/// What one tick answers (synthetic-loop-dia.md, 20): the DM542 signals of
/// the sent rate after the tick, and whether the synthetic Arduino is in
/// FAULT. In FAULT the STEP rate is always 0: the cart stopped in one jump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickAnswer {
    pub signals: Dm542Signals,
    pub fault: bool,
}
