/// The elements in `index.html` this program fills, and the buttons that move
/// the cart.
pub(crate) const STATUS_ELEMENT_ID: &str = "status";
pub(crate) const RAIL_ELEMENT_ID: &str = "rail";
pub(crate) const WHERE_ELEMENT_ID: &str = "where";
pub(crate) const NEXT_ELEMENT_ID: &str = "next";
pub(crate) const RESULT_ELEMENT_ID: &str = "result";
pub(crate) const ROWS_ELEMENT_ID: &str = "rows";
pub(crate) const MOTOR_BUTTON_ID: &str = "motor";
pub(crate) const IDLER_BUTTON_ID: &str = "idler";
pub(crate) const CENTRE_BUTTON_ID: &str = "centre";
pub(crate) const SPEED_ELEMENT_ID: &str = "speed";
pub(crate) const STOP_BUTTON_ID: &str = "stop";
pub(crate) const MEASURE_BUTTON_ID: &str = "measure-here";

/// The pose buttons in each end's box, by stop and pole.
pub(crate) const POSE_IDLER_HANGING_ID: &str = "pose-idler-hanging";
pub(crate) const POSE_IDLER_HORIZONTAL_ID: &str = "pose-idler-horizontal";
pub(crate) const POSE_IDLER_UPRIGHT_ID: &str = "pose-idler-upright";
pub(crate) const POSE_MOTOR_HANGING_ID: &str = "pose-motor-hanging";
pub(crate) const POSE_MOTOR_HORIZONTAL_ID: &str = "pose-motor-horizontal";
pub(crate) const POSE_MOTOR_UPRIGHT_ID: &str = "pose-motor-upright";
pub(crate) const POSE_CENTRE_HANGING_ID: &str = "pose-centre-hanging";

/// The modal's elements, in `index.html`.
pub(crate) const MODAL_ID: &str = "modal";
pub(crate) const MODAL_TITLE_ID: &str = "modal-title";
pub(crate) const MODAL_STEADY_ID: &str = "modal-steady";
pub(crate) const MODAL_STEADY_TEXT_ID: &str = "modal-steady-text";
pub(crate) const MODAL_LIVE_IMAGE_ID: &str = "modal-live-image";
pub(crate) const MODAL_LIVE_BOXES_ID: &str = "modal-live-boxes";
pub(crate) const MODAL_LIVE_TEXT_ID: &str = "modal-live-text";
pub(crate) const MODAL_RUNNING_ID: &str = "modal-running";
pub(crate) const MODAL_SUMMARY_ID: &str = "modal-summary";
pub(crate) const MODAL_FIT_ID: &str = "modal-fit";
pub(crate) const MODAL_PROBLEM_ID: &str = "modal-problem";
pub(crate) const MODAL_GAP_ROW_ID: &str = "modal-gap-row";
pub(crate) const MODAL_GAP_ID: &str = "modal-gap";
pub(crate) const MODAL_GAP_ERROR_ID: &str = "modal-gap-error";
pub(crate) const MODAL_COUNT_ID: &str = "modal-count";
pub(crate) const MODAL_COUNT_HELP_ID: &str = "modal-count-help";
pub(crate) const MODAL_COUNT_ERROR_ID: &str = "modal-count-error";
pub(crate) const MODAL_PROGRESS_ID: &str = "modal-progress";
pub(crate) const MODAL_PROGRESS_TEXT_ID: &str = "modal-progress-text";
pub(crate) const MODAL_IMAGE_ID: &str = "modal-image";
pub(crate) const MODAL_BOXES_ID: &str = "modal-boxes";
pub(crate) const MODAL_SUMMARY_TEXT_ID: &str = "modal-summary-text";
pub(crate) const MODAL_FIT_TEXT_ID: &str = "modal-fit-text";
pub(crate) const MODAL_ACCEPT_FIT_ID: &str = "modal-accept-fit";
pub(crate) const MODAL_PROBLEM_TEXT_ID: &str = "modal-problem-text";
pub(crate) const MODAL_CALIBRATE_ID: &str = "modal-calibrate";
pub(crate) const MODAL_DONE_ID: &str = "modal-done";
pub(crate) const MODAL_RETRY_ID: &str = "modal-retry";
pub(crate) const MODAL_CANCEL_ID: &str = "modal-cancel";
pub(crate) const MODAL_CLOSE_ID: &str = "modal-close";

/// The webserver's routes: `ARDUINO_PINS_ROUTE`, `ARDUINO_SETTINGS_ROUTE` and the
/// `CALIBRATE_*_ROUTE`s in `__live__/orin/rust/crates/webserver/src/constants.rs`.
pub(crate) const PINS_ROUTE: &str = "/arduino/pins";
pub(crate) const SETTINGS_ROUTE: &str = "/arduino/settings";
pub(crate) const MOTOR_ROUTE: &str = "/calibrate/motor";
pub(crate) const IDLER_ROUTE: &str = "/calibrate/idler";
pub(crate) const CENTRE_ROUTE: &str = "/calibrate/centre";
pub(crate) const STOP_ROUTE: &str = "/calibrate/stop";
pub(crate) const STATE_ROUTE: &str = "/calibrate/state";
pub(crate) const MEASURE_ROUTE: &str = "/calibrate/measure";
pub(crate) const MEASURE_STATUS_ROUTE: &str = "/calibrate/measure/status";
pub(crate) const MEASURE_CANCEL_ROUTE: &str = "/calibrate/measure/cancel";
pub(crate) const MEASURE_ACCEPT_ROUTE: &str = "/calibrate/measure/accept";
pub(crate) const FIT_ROUTE: &str = "/calibrate/fit";
pub(crate) const FIT_ACCEPT_ROUTE: &str = "/calibrate/fit/accept";
pub(crate) const LIVE_START_ROUTE: &str = "/live/start";
pub(crate) const LIVE_STOP_ROUTE: &str = "/live/stop";
pub(crate) const LIVE_STATUS_ROUTE: &str = "/live/status";

/// The detector's classes in slot order (`[detector] classes` in the Orin's Python
/// settings), each with the class of its box in `index.html`.
pub(crate) const MARKERS: [(&str, &str); 4] = [
    ("Left", "box-left"),
    ("Right", "box-right"),
    ("Pivot", "box-pivot"),
    ("End", "box-end"),
];

/// HTTP's "service unavailable", which `/arduino/pins` answers during a move.
pub(crate) const STATUS_SERVICE_UNAVAILABLE: u16 = 503;

/// HTTP's "OK", which the board's answer holds when the cart did what was asked.
pub(crate) const STATUS_OK: u16 = 200;

/// HTTP's "bad gateway", for a move whose answer the page could not get.
pub(crate) const STATUS_BAD_GATEWAY: u16 = 502;

/// The drawing's cart, in the drawing's units. At the idler end, on the left, it
/// touches that end's switch, at the motor end the other's, and the centre is halfway.
pub(crate) const CART_WIDTH: i32 = 120;
pub(crate) const CART_AT_IDLER_X: i32 = 108;
pub(crate) const CART_AT_MOTOR_X: i32 = 632;
pub(crate) const CART_AT_CENTRE_X: i32 = 370;
