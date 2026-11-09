/// What the walk stops for (synthetic-loop-dia.md, the loop, 11–21): MuJoCo
/// waits on that timestep until the orchestrator has dealt with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Step n's capture: read the capture state, and wait for the Orin's
    /// reply (11–14).
    Capture(i64),
    /// Step n's send: its pool to the trainer, and wait for the answer
    /// (17–18); then `Walk::command` on continue.
    Send(i64),
    /// The tick answered FAULT: the episode ends on this timestep (*The
    /// rail's ends*); the line owed is step `Walk::owed`'s.
    Fault,
}
