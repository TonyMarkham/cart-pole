/// The settings file, read from the folder the brain runs in, the same file the
/// webserver reads (`DEPLOY.md`, step 9). The one path that can't be in it.
pub(crate) const SETTINGS_PATH: &str = "settings.toml";

/// The frame the detector was trained on and the matrix was fitted to: the
/// camera's 1456 × 1088 halved on each side (`real-loop-dia.md`, *Contracts*,
/// the frame). A frame of any other size is refused, never detected on.
pub(crate) const FRAME_WIDTH: usize = 728;
pub(crate) const FRAME_HEIGHT: usize = 544;
pub(crate) const FRAME_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT;

/// The pool: 6 entries, each its age then Pivot's and End's (x, z)
/// (`real-loop-dia.md`, *Contracts*, the pool).
pub(crate) const ENTRIES: usize = 6;
pub(crate) const ROW: usize = 5;
pub(crate) const POOL_FLOATS: usize = ENTRIES * ROW;

/// A marker not found, in a slot and in a point (`real-loop-dia.md`,
/// *Contracts*, the detection and the points).
pub(crate) const NOT_DETECTED: f64 = -1.0;

/// The detector's classes in slot order: 0 Left, 1 Right, 2 Pivot, 3 End
/// (`AGENTS.md`, *Verified contracts*). Only Pivot and End enter the pool.
pub(crate) const CLASSES: usize = 4;
pub(crate) const PIVOT_CLASS: usize = 2;
pub(crate) const END_CLASS: usize = 3;

/// The command's three bytes (`real-loop-dia.md`, *Contracts*, the command):
/// the sign, then the rate's size as a `u16`, little-endian. Must match the
/// controller firmware's `COMMAND_SIZE`, `COMMAND_NEGATIVE` and
/// `COMMAND_POSITIVE` (`__live__/arduino/rust/crates/controller/src/constants.rs`).
pub(crate) const COMMAND_SIZE: usize = 3;
pub(crate) const COMMAND_NEGATIVE: u8 = 0;
pub(crate) const COMMAND_POSITIVE: u8 = 1;

/// The `status` of the board's FAULT STATE and UNCALIBRATED replies, and the
/// two `result`s that end a run (`real-loop-dia.md`, *The board's states*).
pub(crate) const REPLY_STATUS_CONFLICT: u64 = 409;
pub(crate) const REPLY_FAULT: &str = "fault";
pub(crate) const REPLY_UNCALIBRATED: &str = "uncalibrated";

/// Room for one reply from the board. The controller firmware's `ANSWER_MAX`
/// is 160.
pub(crate) const REPLY_MAX: usize = 256;

/// The GStreamer Data Protocol's packet header, as `gdppay` writes it (GStreamer
/// 1.x, gst-plugins-bad `gst/gdp/dataprotocol.c`, `GST_DP_INIT_HEADER` and
/// `gst_dp_payload_buffer`): 62 bytes, every number big-endian. Byte 0 is the
/// protocol's major version, 1; bytes 4–5 the payload's type; bytes 6–9 the
/// payload's length; bytes 10–17 the buffer's timestamp in nanoseconds.
pub(crate) const GDP_HEADER_LENGTH: usize = 62;
pub(crate) const GDP_MAJOR_VERSION: u8 = 1;
pub(crate) const GDP_TYPE_OFFSET: usize = 4;
pub(crate) const GDP_LENGTH_OFFSET: usize = 6;
pub(crate) const GDP_TIMESTAMP_OFFSET: usize = 10;

/// The payload type of a buffer, a frame; caps are 2 and events 64 and up,
/// and are skipped (`dataprotocol.h`, `GstDPPayloadType`).
pub(crate) const GDP_PAYLOAD_BUFFER: u16 = 1;

/// How long the camera software gets to shut down when asked, s. Killed
/// outright, it can leave NVIDIA's camera service unable to open the camera
/// again (`__live__/orin/python/src/camera.py`, `STOP_TIMEOUT_S`). While it
/// shuts down, the brain looks every `STOP_POLL_MS` whether it has.
pub(crate) const CAMERA_STOP_TIMEOUT_S: u64 = 5;
pub(crate) const STOP_POLL_MS: u64 = 50;

/// How long the worker gets to end once its input pipe is closed, ms, before
/// it is killed. Ending is one read returning nothing, so a second is plenty.
pub(crate) const WORKER_STOP_TIMEOUT_MS: u64 = 1_000;

/// How much longer than one pass of the loop `idle` waits for the loop to
/// stop, ms. A pass is at most `frame_timeout_ms` of waiting for a frame plus
/// the worker's answers; this covers the rest. The camera's closing is not
/// waited for: STOP's zeros must follow the last command at once
/// (`real-loop-dia.md`, *The run and idle states*, STOP), and on 2026-11-07
/// they followed it five seconds late while the camera closed, and the board
/// held the last command, full speed, to the soft limit.
pub(crate) const IDLE_WAIT_MARGIN_MS: u64 = 1_000;

/// GStreamer's launcher, and how a process is asked to stop: `kill -INT` sends
/// the interrupt a Ctrl-C would, the signal the camera software shuts down
/// cleanly on, without a signals library of the brain's own.
pub(crate) const GST_LAUNCH: &str = "gst-launch-1.0";
pub(crate) const KILL: &str = "kill";
pub(crate) const KILL_INTERRUPT: &str = "-INT";

/// The camera pipeline's parts, as `gst-launch-1.0` takes them: the Python
/// camera's pipeline (`__live__/orin/python/src/camera.py`) with `gdppay`
/// before the sink. `-q` keeps GStreamer's progress messages off the output;
/// `!` links one element to the next; the two caps strings say what each link
/// carries.
pub(crate) const GST_QUIET: &str = "-q";
pub(crate) const GST_LINK: &str = "!";
pub(crate) const GST_SOURCE: &str = "nvarguscamerasrc";
pub(crate) const GST_DO_TIMESTAMP: &str = "do-timestamp=true";
pub(crate) const GST_SENSOR_ID: &str = "sensor-id";
pub(crate) const GST_SENSOR_MODE: &str = "sensor-mode";
pub(crate) const GST_TNR_MODE: &str = "tnr-mode";
pub(crate) const GST_EE_MODE: &str = "ee-mode";
pub(crate) const GST_EXPOSURE_RANGE: &str = "exposuretimerange";
pub(crate) const GST_GAIN_RANGE: &str = "gainrange";
/// The ISP's digital gain, locked at 1 so it adds nothing to the sensor's gain.
pub(crate) const GST_ISP_GAIN_LOCKED: &str = "ispdigitalgainrange=1 1";
pub(crate) const GST_CAPS_NVMM: &str = "video/x-raw(memory:NVMM)";
pub(crate) const GST_CONVERTER: &str = "nvvidconv";
pub(crate) const GST_CAPS_RAW: &str = "video/x-raw";
/// One byte per pixel, greyscale.
pub(crate) const GST_FORMAT_GREY: &str = "GRAY8";
pub(crate) const GST_PAYLOADER: &str = "gdppay";
pub(crate) const GST_SINK: &str = "fdsink";
/// File descriptor 1 is the process's standard output, the pipe the brain reads.
pub(crate) const GST_SINK_STDOUT: &str = "fd=1";

/// The socket the commands leave from: any of the Orin's addresses, and a port
/// the system picks, since the board answers whoever sent the latest datagram.
pub(crate) const ANY_ADDRESS: &str = "0.0.0.0";
pub(crate) const ANY_PORT: u16 = 0;

/// A little-endian `f64` on the worker's pipe, bytes.
pub(crate) const F64_BYTES: usize = 8;

/// The worker's two requests, the first byte of each (`real-loop-dia.md`,
/// *Contracts*, the brain's worker). Must match `brain_worker.py`'s
/// `FRAME_REQUEST` and `POLICY_REQUEST`.
pub(crate) const WORKER_FRAME_REQUEST: u8 = b'F';
pub(crate) const WORKER_POLICY_REQUEST: u8 = b'P';

/// A detection's five numbers, each a little-endian `f32`: centre x, centre y,
/// width, height, confidence (`real-loop-dia.md`, *Contracts*, the brain's
/// worker).
pub(crate) const DETECTION_FLOATS: usize = 5;
pub(crate) const DETECTIONS_BYTES: usize = CLASSES * DETECTION_FLOATS * 4;

/// The brain's own requests, which the webserver forwards to
/// (`real-loop-dia.md`, *Contracts*, the brain's requests). Must match the
/// webserver's `BRAIN_RUN_PATH`, `BRAIN_IDLE_PATH` and `BRAIN_STATUS_PATH`.
pub(crate) const RUN_ROUTE: &str = "/run";
pub(crate) const IDLE_ROUTE: &str = "/idle";
pub(crate) const STATUS_ROUTE: &str = "/status";

/// `calibrate_service.py`'s question "is the camera yours?"
/// (`calibrate_service.py`, `do_GET`).
pub(crate) const SERVICE_BUSY_PATH: &str = "/busy";

/// The log file in each run's folder (`real-loop-dia.md`, *The log*).
pub(crate) const FRAMES_FILE: &str = "frames.jsonl";

/// Undoing the lens's bend has no formula, so it is found by repeating until
/// the answer, bent back, lands within this of the pixel asked about, or this
/// many repeats have run: the same stopping point as the Python's
/// `lens.py` `CRITERIA`, which the matrix was fitted with.
pub(crate) const LENS_EPS_PX: f64 = 1e-12;
pub(crate) const LENS_ITERATIONS: usize = 100;

/// Nanoseconds in a millisecond, for GStreamer's stamps.
pub(crate) const NANOS_PER_MILLI: f64 = 1_000_000.0;
