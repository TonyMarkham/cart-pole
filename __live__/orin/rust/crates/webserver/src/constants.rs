pub const ADDRESS: &str = "0.0.0.0:8080";
pub(crate) const VIEWER_DIR: &str = "pages/viewer";
pub(crate) const ARDUINO_PAGE_DIR: &str = "pages/arduino";
pub(crate) const SETTINGS_PATH: &str = "settings.toml";
pub(crate) const HELLO_ROUTE: &str = "/";
pub(crate) const CALIBRATE_ROUTE: &str = "/calibrate";
pub(crate) const VIEWER_ROUTE: &str = "/viewer";
pub(crate) const WS_ROUTE: &str = "/ws";
pub(crate) const IMAGE_ROUTE: &str = "/image";

/// Must match the page's `public_url`, in its `Trunk.toml`.
pub(crate) const ARDUINO_PAGE_ROUTE: &str = "/arduino";

/// Must match the page's `PINS_ROUTE` and `SETTINGS_ROUTE`.
pub(crate) const ARDUINO_PINS_ROUTE: &str = "/arduino/pins";
pub(crate) const ARDUINO_SETTINGS_ROUTE: &str = "/arduino/settings";

/// The datagram that asks the Arduino for its pin levels: must match the
/// firmware's `PINS_REQUEST`.
pub(crate) const PINS_REQUEST: &[u8] = b"pins";

/// Room for the Arduino's answer, `{"d2":0,"d3":0,"d4":1,"d5":1,"d6":1}`.
pub(crate) const PINS_ANSWER_MAX: usize = 64;

pub(crate) const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
