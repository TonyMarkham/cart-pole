/// The elements in `index.html` this program fills.
pub(crate) const BUTTON_ID: &str = "toggle";
pub(crate) const STATUS_ID: &str = "status";
pub(crate) const IMAGE_ID: &str = "image";
pub(crate) const BOXES_ID: &str = "boxes";
pub(crate) const ROWS_ID: &str = "rows";

/// The webserver's routes: the `LIVE_*_ROUTE`s and `CALIBRATE_STATE_ROUTE` in
/// `__live__/orin/rust/crates/webserver/src/constants.rs`.
pub(crate) const START_ROUTE: &str = "/live/start";
pub(crate) const STOP_ROUTE: &str = "/live/stop";
pub(crate) const STATUS_ROUTE: &str = "/live/status";
/// Says how often to look, `[calibration] poll_ms` in the webserver's settings.
pub(crate) const STATE_ROUTE: &str = "/calibrate/state";

/// The detector's classes in slot order (`[detector] classes` in the Orin's Python
/// settings), each with the class of its box in `index.html`.
pub(crate) const MARKERS: [(&str, &str); 4] = [
    ("Left", "box-left"),
    ("Right", "box-right"),
    ("Pivot", "box-pivot"),
    ("End", "box-end"),
];
