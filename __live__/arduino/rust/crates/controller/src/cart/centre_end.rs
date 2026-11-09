use crate::cart::rail_end::RailEnd;

/// How a move to the centre ended. Each is one answer to the Orin
/// (`Answer::centre`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CentreEnd {
    /// The cart stopped at the centre.
    Centred,
    /// That end's switch tripped on the way, so the count was wrong: the rail
    /// is forgotten.
    Tripped(RailEnd),
    /// A `Stop` arrived during the move.
    Stopped,
    /// The rail isn't known yet, so there is no centre: the cart didn't move.
    NotReady,
    /// The speed asked for isn't one the move may run at: the cart didn't
    /// move.
    BadSpeed,
    /// Both switches read pressed at the start: the cart didn't move.
    Refused,
    /// A command with a bad sign byte arrived during the move: the cart was
    /// stopped, and the board is in FAULT.
    Fault,
}
