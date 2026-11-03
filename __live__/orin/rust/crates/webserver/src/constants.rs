pub const ADDRESS: &str = "0.0.0.0:8080";
pub(crate) const VIEWER_DIR: &str = "pages/viewer";
pub(crate) const ARDUINO_PAGE_DIR: &str = "pages/arduino";
pub(crate) const SETTINGS_PATH: &str = "settings.toml";
pub(crate) const HELLO_ROUTE: &str = "/";
pub(crate) const CALIBRATE_PAGE_DIR: &str = "pages/calibrate";
pub(crate) const VIEWER_ROUTE: &str = "/viewer";
pub(crate) const WS_ROUTE: &str = "/ws";
pub(crate) const IMAGE_ROUTE: &str = "/image";

/// Must match the page's `public_url`, in its `Trunk.toml`.
pub(crate) const ARDUINO_PAGE_ROUTE: &str = "/arduino";

/// Must match the page's `PINS_ROUTE` and `SETTINGS_ROUTE`.
pub(crate) const ARDUINO_PINS_ROUTE: &str = "/arduino/pins";
pub(crate) const ARDUINO_SETTINGS_ROUTE: &str = "/arduino/settings";

/// Must match the page's `public_url`, in its `Trunk.toml`.
pub(crate) const CALIBRATE_PAGE_ROUTE: &str = "/calibrate";

/// Must match the routes in the page's `Travel`.
pub(crate) const CALIBRATE_IDLER_ROUTE: &str = "/calibrate/idler";
pub(crate) const CALIBRATE_MOTOR_ROUTE: &str = "/calibrate/motor";
pub(crate) const CALIBRATE_CENTRE_ROUTE: &str = "/calibrate/centre";
pub(crate) const CALIBRATE_STOP_ROUTE: &str = "/calibrate/stop";

/// Must match the page's routes, in its `constants.rs`.
pub(crate) const CALIBRATE_STATE_ROUTE: &str = "/calibrate/state";
pub(crate) const CALIBRATE_MEASURE_ROUTE: &str = "/calibrate/measure";
pub(crate) const CALIBRATE_MEASURE_STATUS_ROUTE: &str = "/calibrate/measure/status";
pub(crate) const CALIBRATE_MEASURE_CANCEL_ROUTE: &str = "/calibrate/measure/cancel";
pub(crate) const CALIBRATE_MEASURE_ACCEPT_ROUTE: &str = "/calibrate/measure/accept";
pub(crate) const CALIBRATE_FIT_ROUTE: &str = "/calibrate/fit";
pub(crate) const CALIBRATE_FIT_ACCEPT_ROUTE: &str = "/calibrate/fit/accept";

/// Must match the live page's `public_url`, in its `Trunk.toml`.
pub(crate) const LIVE_PAGE_ROUTE: &str = "/live";
pub(crate) const LIVE_PAGE_DIR: &str = "pages/live";

/// Must match the live page's routes, in its `constants.rs`.
pub(crate) const LIVE_START_ROUTE: &str = "/live/start";
pub(crate) const LIVE_STOP_ROUTE: &str = "/live/stop";
pub(crate) const LIVE_STATUS_ROUTE: &str = "/live/status";

/// Must match the lens page's `public_url`, in its `Trunk.toml`.
pub(crate) const LENS_PAGE_ROUTE: &str = "/lens";
pub(crate) const LENS_PAGE_DIR: &str = "pages/lens";

/// Must match the lens page's routes, in its `constants.rs`.
pub(crate) const LENS_START_ROUTE: &str = "/lens/start";
pub(crate) const LENS_STOP_ROUTE: &str = "/lens/stop";
pub(crate) const LENS_STATUS_ROUTE: &str = "/lens/status";
pub(crate) const LENS_SKIP_ROUTE: &str = "/lens/skip";
pub(crate) const LENS_RESTART_ROUTE: &str = "/lens/restart";
pub(crate) const LENS_SOLVE_ROUTE: &str = "/lens/solve";
pub(crate) const LENS_STILL_ROUTE: &str = "/lens/still";

/// The datagram that asks the Arduino for its pin levels: must match the
/// firmware's `PINS_REQUEST`.
pub(crate) const PINS_REQUEST: &[u8] = b"pins";

/// The datagrams that move the cart: must match the firmware's `IDLER_REQUEST`,
/// `MOTOR_REQUEST` and `CENTRE_REQUEST`.
pub(crate) const IDLER_REQUEST: &str = "CalibrateIdlerTravelLimit";
pub(crate) const MOTOR_REQUEST: &str = "CalibrateMotorTravelLimit";
pub(crate) const CENTRE_REQUEST: &str = "MoveToCentre";

/// The datagram that ends a move: must match the firmware's `STOP_FRAME_END`.
pub(crate) const STOP_REQUEST: &[u8] = b"Stop";

/// Room for the Arduino's answer, `{"d2":0,"d3":0,"d4":1,"d5":1,"d6":1}`.
pub(crate) const PINS_ANSWER_MAX: usize = 64;

/// Room for a move's answer. The firmware's `SEEK_ANSWER_MAX` is 160.
pub(crate) const MOVE_ANSWER_MAX: usize = 256;

pub(crate) const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
