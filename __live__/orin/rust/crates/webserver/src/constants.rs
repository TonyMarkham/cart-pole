/// The settings file, read from the folder the server runs in (the README,
/// deployment step 9). The one path that can't be in it.
pub(crate) const SETTINGS_PATH: &str = "settings.toml";

/// The hello page, which links to every other page.
pub(crate) const HELLO_ROUTE: &str = "/";

/// The built pages, relative to the folder the server runs in. Each must match
/// `dist` in that page's `Trunk.toml`, under `../wasm`, which writes it.
pub(crate) const VIEWER_DIR: &str = "pages/viewer";
pub(crate) const ARDUINO_PAGE_DIR: &str = "pages/arduino";
pub(crate) const CALIBRATE_PAGE_DIR: &str = "pages/calibrate";
pub(crate) const LIVE_PAGE_DIR: &str = "pages/live";
pub(crate) const LENS_PAGE_DIR: &str = "pages/lens";
pub(crate) const RUN_PAGE_DIR: &str = "pages/run";

/// The viewer page: must match its `public_url`, in its `Trunk.toml`.
pub(crate) const VIEWER_ROUTE: &str = "/viewer";

/// The viewer's WebSocket: must match the viewer page's `WS_ROUTE`.
pub(crate) const WS_ROUTE: &str = "/ws";

/// Where `image-push` posts a PNG: must match its `IMAGE_ROUTE`.
pub(crate) const IMAGE_ROUTE: &str = "/image";

/// Where the viewer page's Refresh button posts: must match the viewer page's
/// `REFRESH_ROUTE`. Under `/image`, not `/viewer`, which serves the page's
/// files.
pub(crate) const IMAGE_REFRESH_ROUTE: &str = "/image/refresh";

/// The live view's own paths on `calibrate_service.py` (its `do_GET` and
/// `do_POST`), which the refresh route asks.
pub(crate) const SERVICE_LIVE_START: &str = "/live/start";
pub(crate) const SERVICE_LIVE_STOP: &str = "/live/stop";
pub(crate) const SERVICE_LIVE_STATUS: &str = "/live/status";

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

/// Must match the live page's routes, in its `constants.rs`.
pub(crate) const LIVE_START_ROUTE: &str = "/live/start";
pub(crate) const LIVE_STOP_ROUTE: &str = "/live/stop";
pub(crate) const LIVE_STATUS_ROUTE: &str = "/live/status";

/// Must match the lens page's `public_url`, in its `Trunk.toml`.
pub(crate) const LENS_PAGE_ROUTE: &str = "/lens";

/// Must match the lens page's routes, in its `constants.rs`.
pub(crate) const LENS_START_ROUTE: &str = "/lens/start";
pub(crate) const LENS_STOP_ROUTE: &str = "/lens/stop";
pub(crate) const LENS_STATUS_ROUTE: &str = "/lens/status";
pub(crate) const LENS_SKIP_ROUTE: &str = "/lens/skip";
pub(crate) const LENS_RESTART_ROUTE: &str = "/lens/restart";
pub(crate) const LENS_SOLVE_ROUTE: &str = "/lens/solve";
pub(crate) const LENS_STILL_ROUTE: &str = "/lens/still";

/// Must match the run page's `public_url`, in its `Trunk.toml`. Its routes
/// below sit under it, as the calibrate page's do.
pub(crate) const RUN_PAGE_ROUTE: &str = "/run";

/// Must match the run page's routes, in its `constants.rs`
/// (`real-orin-plan.md`, O9).
pub(crate) const RUN_START_ROUTE: &str = "/run/start";
pub(crate) const RUN_STOP_ROUTE: &str = "/run/stop";
pub(crate) const RUN_STATUS_ROUTE: &str = "/run/status";

/// The brain's own requests, which the run routes forward to: must match
/// `RUN_ROUTE`, `IDLE_ROUTE` and `STATUS_ROUTE` in the brain
/// (`crates/brain/src/constants.rs`).
pub(crate) const BRAIN_RUN_PATH: &str = "/run";
pub(crate) const BRAIN_IDLE_PATH: &str = "/idle";
pub(crate) const BRAIN_STATUS_PATH: &str = "/status";

/// The command 0, which brings the cart to rest: the 3-byte command of
/// `__documentation__/real-loop-dia.md`, *Contracts*, a positive sign (1) and a size of 0,
/// little-endian. STOP sends it; the board takes a command only in CALIBRATED
/// and MOVING, and ramps to rest on it.
pub(crate) const COMMAND_ZERO: [u8; 3] = [1, 0, 0];

/// The datagram that asks the Arduino for its pin levels: must match
/// `PINS_REQUEST` in the calibration firmware
/// (`__live__/arduino/rust/crates/calibration/src/constants.rs`).
pub(crate) const PINS_REQUEST: &[u8] = b"pins";

/// The datagrams that move the cart: must match `SEEK_IDLER_REQUEST`,
/// `SEEK_MOTOR_REQUEST` and `CENTRE_REQUEST` in the controller firmware
/// (`__live__/arduino/rust/crates/controller/src/constants.rs`).
pub(crate) const IDLER_REQUEST: &str = "CalibrateIdlerTravelLimit";
pub(crate) const MOTOR_REQUEST: &str = "CalibrateMotorTravelLimit";
pub(crate) const CENTRE_REQUEST: &str = "MoveToCentre";

/// The datagram that ends a move: must match the controller firmware's
/// `STOP_REQUEST`.
pub(crate) const STOP_REQUEST: &[u8] = b"Stop";

/// The datagram that asks the board its state, for `/run`'s Run rule
/// (`real-open-issues.md`, issue 7): must match `STATE_REQUEST` in the
/// controller firmware. At 5 bytes it can't be taken for a 3-byte command.
pub(crate) const STATE_REQUEST: &[u8] = b"state";

/// Room for the answer to `state`. The controller firmware's `ANSWER_MAX` is
/// 160.
pub(crate) const STATE_ANSWER_MAX: usize = 256;

/// Room for the Arduino's answer, `{"d2":0,"d3":0,"d4":1,"d5":1,"d6":1}`.
pub(crate) const PINS_ANSWER_MAX: usize = 64;

/// Room for a move's answer. The controller firmware's `ANSWER_MAX` is 160.
pub(crate) const MOVE_ANSWER_MAX: usize = 256;

/// The `status` in a move's answer when the cart did what was asked: HTTP's
/// "OK", the controller firmware's `STATUS_OK`.
pub(crate) const STATUS_OK: u64 = 200;

/// Milliseconds in a second, for the time a centring takes at a speed.
pub(crate) const MILLIS_PER_SECOND: u64 = 1_000;

/// The 8 bytes every PNG file starts with (the W3C PNG specification, 5.2,
/// *PNG signature*).
/// Must match `image-push`'s `PNG_SIGNATURE`.
pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The two fields of the live view's status the refresh route reads
/// (`calibrate_service.py`, `live_status`): whether the view is on, and the
/// latest frame as base64 JPEG, null until the view has one.
pub(crate) const LIVE_STATUS_ON: &str = "on";
pub(crate) const LIVE_STATUS_FRAME: &str = "frame";

/// The content type of a JSON answer passed on unchanged.
pub(crate) const JSON_CONTENT_TYPE: &str = "application/json";
