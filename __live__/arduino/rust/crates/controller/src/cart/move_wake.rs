use crate::cart::move_end::MoveEnd;

/// What woke the board during a move with something to act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveWake {
    /// The move is over.
    Ended(MoveEnd),
    /// The cart has moved `release_pulses` off the switch it started on, so
    /// that switch should read released now.
    LeftSwitch,
}
