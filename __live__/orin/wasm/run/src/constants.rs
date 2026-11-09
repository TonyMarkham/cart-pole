/// The elements in `index.html` this program fills or listens to.
pub(crate) const RUN_ID: &str = "run";
pub(crate) const CENTRE_ID: &str = "centre";
pub(crate) const STOP_ID: &str = "stop";
pub(crate) const WHY_ID: &str = "why";
pub(crate) const RESULT_ID: &str = "result";
pub(crate) const STATE_ID: &str = "state";
pub(crate) const MESSAGE_ID: &str = "message";
pub(crate) const FRAMES_ID: &str = "frames";
pub(crate) const RUN_DIR_ID: &str = "run-dir";
pub(crate) const BOARD_ID: &str = "board";

/// The webserver's routes: the `RUN_*_ROUTE`s in
/// `__live__/orin/rust/crates/webserver/src/constants.rs`.
pub(crate) const START_ROUTE: &str = "/run/start";
pub(crate) const STOP_ROUTE: &str = "/run/stop";
pub(crate) const STATUS_ROUTE: &str = "/run/status";

/// The calibrate page's move to the centre and its Stop, which Re-centre and
/// STOP use: `CALIBRATE_CENTRE_ROUTE` and `CALIBRATE_STOP_ROUTE` in the
/// webserver's `constants.rs`.
pub(crate) const CENTRE_ROUTE: &str = "/calibrate/centre";
pub(crate) const MOVE_STOP_ROUTE: &str = "/calibrate/stop";

/// The brain's two states, its `state` (`real-loop-dia.md`, *Contracts*, the
/// brain's requests).
pub(crate) const BRAIN_RUN: &str = "run";
pub(crate) const BRAIN_IDLE: &str = "idle";

/// The board's answers to `state`, its `result` (`real-loop-dia.md`, *The
/// board's states*; the controller firmware's `State::name`).
pub(crate) const BOARD_CALIBRATED: &str = "calibrated";
pub(crate) const BOARD_UNCALIBRATED: &str = "uncalibrated";
pub(crate) const BOARD_CALIBRATING: &str = "calibrating";
pub(crate) const BOARD_FAULT: &str = "fault";

/// The `result` of a move to the centre that arrived (the controller
/// firmware's `Answer::centre`).
pub(crate) const MOVE_CENTRED: &str = "centred";
