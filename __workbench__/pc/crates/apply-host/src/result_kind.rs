/// What the R3 saw after one mark (`apply_timer.ino`, its header).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResultKind {
    /// The new rate began after the mark: the delay is gap 8.
    Applied,
    /// The rate changed before the mark: no command can cause that, so it
    /// is counted, not timed.
    Early,
    /// The rate never changed within the R3's timeout.
    None,
}
