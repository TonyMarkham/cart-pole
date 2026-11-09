use crate::cart::{centre_speed::CentreSpeed, rail_end::RailEnd};

/// A move of the calibration: what any calibration button on the Orin's
/// calibration page asks the board for (real-open-issues.md, issue 5, step 1,
/// *Follows*). Each puts the board in CALIBRATING.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CalibrationMove {
    /// Seek that end of the rail.
    Seek(RailEnd),
    /// Move to the centre of the rail, at that speed.
    Centre(CentreSpeed),
}
