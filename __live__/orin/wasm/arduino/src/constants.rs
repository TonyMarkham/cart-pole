pub(crate) const BOARD_ELEMENT_ID: &str = "board";
pub(crate) const ROWS_ELEMENT_ID: &str = "rows";
pub(crate) const CONNECTION_ELEMENT_ID: &str = "connection";
pub(crate) const POLL_ELEMENT_ID: &str = "poll-ms";

/// The webserver's routes: `ARDUINO_PINS_ROUTE` and `ARDUINO_SETTINGS_ROUTE`
/// in `__live__/orin/rust/crates/webserver/src/constants.rs`.
pub(crate) const PINS_ROUTE: &str = "/arduino/pins";
pub(crate) const SETTINGS_ROUTE: &str = "/arduino/settings";

/// The headers' rows, in the drawing's units.
pub(crate) const TOP_Y: i32 = 38;
pub(crate) const BOTTOM_Y: i32 = 558;

/// Where the board sits in the drawing, leaving room for the cards around it.
pub(crate) const BOARD_X: i32 = 30;
pub(crate) const BOARD_Y: i32 = 190;
