pub(crate) mod event;

// ---------------------------------------------------------------------------------------------- //

use crate::state::event::Event;

/// What the board may do as things stand: the five states the user decided
/// (2026-11-04; CALIBRATING added 2026-11-05), drawn in
/// `real-open-issues.md`, issue 5, step 1. The board knows no run: a run is
/// the Orin's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    /// The board knows nothing of the rail. A reset forgets the rail, so
    /// every start is here. No command is applied: without the ends' cached
    /// trip positions there is no soft limit.
    Uncalibrated,
    /// A calibration has begun: a seek to an end, or the move to the centre.
    /// The board stays here between those moves, and after a seek that
    /// fails or is stopped, until the move to the centre succeeds.
    Calibrating,
    /// The rail is known and the cart is at rest. Commands are applied.
    Calibrated,
    /// The board is sending pulses to the motor driver, outside a
    /// calibration.
    Moving,
    /// The cart entered a protected end zone while moving, or a limit switch
    /// was pressed in CALIBRATED or MOVING. The cart is stopped, and no
    /// command is applied. Only a reset leaves this state,
    /// back to `Uncalibrated`.
    Fault,
}

impl State {
    /// Every start: a reset forgets the rail.
    pub const START: State = State::Uncalibrated;

    /// The state after `event`. Only the arrows of issue 5's diagram change
    /// the state; any other event leaves it where it is:
    ///
    /// | From           | Event               | To             |
    /// |----------------|---------------------|----------------|
    /// | `Uncalibrated` | `StartCalibrating`  | `Calibrating`  |
    /// | `Calibrated`   | `StartCalibrating`  | `Calibrating`  |
    /// | `Calibrating`  | `FinishCalibrating` | `Calibrated`   |
    /// | `Calibrated`   | `MoveRequested`     | `Moving`       |
    /// | `Moving`       | `FinishMoving`      | `Calibrated`   |
    /// | `Moving`       | `EndZoneEntered`    | `Fault`        |
    /// | `Calibrated`   | `SwitchTripped`     | `Fault`        |
    /// | `Moving`       | `SwitchTripped`     | `Fault`        |
    /// | any            | `BadSign`           | `Fault`        |
    /// | `Fault`        | `Resetting`         | `Uncalibrated` |
    ///
    /// So no event leads from `Calibrating` to `Uncalibrated` (the user,
    /// 2026-11-05), only a bad sign byte leads from it to `Fault` (the user,
    /// 2026-11-06), any switch press in `Calibrated` or `Moving` is a fault
    /// (the user, 2026-11-07), and only `Resetting` leads out of `Fault`.
    pub const fn next(self, event: Event) -> State {
        match (self, event) {
            (State::Uncalibrated | State::Calibrated, Event::StartCalibrating) => {
                State::Calibrating
            }
            (State::Calibrating, Event::FinishCalibrating) => State::Calibrated,
            (State::Calibrated, Event::MoveRequested) => State::Moving,
            (State::Moving, Event::FinishMoving) => State::Calibrated,
            (State::Moving, Event::EndZoneEntered) => State::Fault,
            (State::Calibrated | State::Moving, Event::SwitchTripped) => State::Fault,
            (State::Fault, Event::Resetting) => State::Uncalibrated,
            (_, Event::BadSign) => State::Fault,
            (state, _) => state,
        }
    }

    /// The state's name in an answer: `OK STATUS uncalibrated`, and later
    /// the WiFi `state` reply's `result` (real-open-issues.md, issue 7).
    pub const fn name(self) -> &'static str {
        match self {
            State::Uncalibrated => "uncalibrated",
            State::Calibrating => "calibrating",
            State::Calibrated => "calibrated",
            State::Moving => "moving",
            State::Fault => "fault",
        }
    }
}
