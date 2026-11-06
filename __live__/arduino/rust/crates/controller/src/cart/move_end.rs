use crate::cart::rail_end::RailEnd;

/// What ended a move, before it is told as a seek's or the centre's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveEnd {
    /// That end's switch tripped, or still read pressed once the cart should
    /// have left it.
    Switch(RailEnd),
    /// The pulse count the move was armed with was reached: the hardware
    /// stopped the pulses.
    Count,
    /// A `Stop` arrived.
    Stopped,
    /// The move's time limit passed.
    TimedOut,
}
