use crate::cart::centre_end::CentreEnd;

/// What woke the board during the move to the centre with something to act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CentreWake {
    /// A 2 ms tick: the ramp takes its next step.
    Tick,
    /// A `Stop` arrived: the ramp brakes the cart to rest.
    Stop,
    /// The move is over.
    Ended(CentreEnd),
}
