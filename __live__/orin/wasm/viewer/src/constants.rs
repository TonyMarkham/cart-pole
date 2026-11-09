/// The `<img>` in `index.html` each image is shown in.
pub(crate) const IMAGE_ELEMENT_ID: &str = "image";

/// The Refresh button in `index.html`, and the text beside it that says what
/// the refresh is doing.
pub(crate) const REFRESH_BUTTON_ID: &str = "refresh";
pub(crate) const STATUS_ELEMENT_ID: &str = "status";

/// The webserver's WebSocket that pushes the images: `WS_ROUTE` in
/// `__live__/orin/rust/crates/webserver/src/constants.rs`.
pub(crate) const WS_ROUTE: &str = "/ws";

/// Where the Refresh button posts, which puts a fresh camera frame on every
/// viewer: `IMAGE_REFRESH_ROUTE` in the webserver's `constants.rs`.
pub(crate) const REFRESH_ROUTE: &str = "/image/refresh";

/// What the status says while the webserver takes the frame: the camera takes
/// a second or two to start.
pub(crate) const STATUS_REFRESHING: &str = "taking a frame…";

/// The key of a refusal's words in the webserver's JSON answer.
pub(crate) const REFUSAL_MESSAGE: &str = "message";

/// The request's method: the refresh route takes a POST, since it changes
/// what every viewer shows.
pub(crate) const POST_METHOD: &str = "POST";

/// The browser's event for a button being pressed.
pub(crate) const CLICK_EVENT: &str = "click";
