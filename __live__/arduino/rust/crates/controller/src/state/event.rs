/// Something that happened to the board, as the board sees it, which may
/// change its state (`State::next`). Each is an arrow's label in
/// `real-open-issues.md`, issue 5's state diagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    /// A calibration move was asked for: a seek to an end, or the move to the
    /// centre. To the board, any calibration button on the Orin's calibration
    /// page is one of these; the page's STOP and its measuring buttons send
    /// the board no move (issue 5, *Follows*).
    StartCalibrating,
    /// The move to the centre ended at the centre.
    FinishCalibrating,
    /// A move was asked for outside a calibration: a command with a non-zero
    /// rate, or a `MoveBy` from the homography page (the user, 2026-11-05,
    /// issue 13).
    MoveRequested,
    /// The pulses stopped: the cart is at rest.
    FinishMoving,
    /// The cart entered a protected end zone, the `runtime_end_margin` pulses
    /// between an end's stop point and its cached trip position. The only
    /// cause of a fault (the user, 2026-11-05: "ONLY ONLY ONLY entering the
    /// protected end zone is a fault").
    EndZoneEntered,
    /// A reset of the board: the only way out of FAULT, back to UNCALIBRATED
    /// (issue 5, step 1).
    Resetting,
}
