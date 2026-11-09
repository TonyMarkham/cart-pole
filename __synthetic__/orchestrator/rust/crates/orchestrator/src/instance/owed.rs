use crate::instance::held::Held;

/// The line an instance owes the trainer (17): the held step whose send
/// came, or after a FAULT the step after the last sent (*The rail's ends*),
/// with the time it went out, ms from the episode's start.
pub struct Owed {
    pub held: Held,
    pub fault: bool,
    pub at_ms: f64,
}
