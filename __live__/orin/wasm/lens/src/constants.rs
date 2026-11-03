/// The elements in `index.html` this program fills or listens to.
pub(crate) const TOGGLE_ID: &str = "toggle";
pub(crate) const SKIP_ID: &str = "skip";
pub(crate) const RESTART_ID: &str = "restart";
pub(crate) const SOLVE_ID: &str = "solve";
pub(crate) const PICTURE_ID: &str = "picture";
pub(crate) const STATUS_ID: &str = "status";
pub(crate) const IMAGE_ID: &str = "image";
pub(crate) const OVERLAY_ID: &str = "overlay";
pub(crate) const RESULT_ID: &str = "result";
/// The progress bars' rows; each bar's parts add `-label`, `-fill` and `-mark`.
pub(crate) const PROGRESS_ID: &str = "progress";
pub(crate) const TURN_ID: &str = "turn";
pub(crate) const HOLD_ID: &str = "hold";

/// The webserver's routes: the `LENS_*_ROUTE`s and `CALIBRATE_STATE_ROUTE` in
/// `__live__/orin/rust/crates/webserver/src/constants.rs`.
pub(crate) const START_ROUTE: &str = "/lens/start";
pub(crate) const STOP_ROUTE: &str = "/lens/stop";
pub(crate) const STATUS_ROUTE: &str = "/lens/status";
pub(crate) const SKIP_ROUTE: &str = "/lens/skip";
pub(crate) const RESTART_ROUTE: &str = "/lens/restart";
pub(crate) const SOLVE_ROUTE: &str = "/lens/solve";
pub(crate) const STILL_ROUTE: &str = "/lens/still";
/// Says how often to look, `[calibration] poll_ms` in the webserver's settings.
pub(crate) const STATE_ROUTE: &str = "/calibrate/state";
