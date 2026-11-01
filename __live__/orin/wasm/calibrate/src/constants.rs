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

/// The webserver's routes: `ARDUINO_PINS_ROUTE`, `ARDUINO_SETTINGS_ROUTE` and the
/// `CALIBRATE_*_ROUTE`s in `__live__/orin/rust/crates/webserver/src/constants.rs`.
pub(crate) const PINS_ROUTE: &str = "/arduino/pins";
pub(crate) const SETTINGS_ROUTE: &str = "/arduino/settings";
pub(crate) const MOTOR_ROUTE: &str = "/calibrate/motor";
pub(crate) const IDLER_ROUTE: &str = "/calibrate/idler";
pub(crate) const CENTRE_ROUTE: &str = "/calibrate/centre";
pub(crate) const STOP_ROUTE: &str = "/calibrate/stop";

/// HTTP's "service unavailable", which `/arduino/pins` answers during a move.
pub(crate) const STATUS_SERVICE_UNAVAILABLE: u16 = 503;

/// HTTP's "OK", which the board's answer holds when the cart did what was asked.
pub(crate) const STATUS_OK: u16 = 200;

/// HTTP's "bad gateway", for a move whose answer the page could not get.
pub(crate) const STATUS_BAD_GATEWAY: u16 = 502;

/// The drawing's cart, in the drawing's units. At the motor end it touches that
/// end's switch, at the idler end the other's, and the centre is halfway.
pub(crate) const CART_WIDTH: i32 = 120;
pub(crate) const CART_AT_MOTOR_X: i32 = 108;
pub(crate) const CART_AT_IDLER_X: i32 = 632;
pub(crate) const CART_AT_CENTRE_X: i32 = 370;
