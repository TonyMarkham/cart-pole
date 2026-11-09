//! Fixed facts, copied from the brain's `constants.rs`
//! (`__live__/orin/rust/crates/brain`), so the loop timed here is the brain's.

/// The frame the brain reads: the camera's 1456 × 1088 halved on each side,
/// one byte per pixel (the brain's `FRAME_WIDTH`, `FRAME_HEIGHT`).
pub(crate) const FRAME_WIDTH: usize = 728;
pub(crate) const FRAME_HEIGHT: usize = 544;
pub(crate) const FRAME_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT;

/// The pool: 6 entries, each its age then Pivot's and End's (x, z) (the
/// brain's `ENTRIES`, `ROW`, `POOL_FLOATS`).
pub(crate) const ENTRIES: usize = 6;
pub(crate) const ROW: usize = 5;
pub(crate) const POOL_FLOATS: usize = ENTRIES * ROW;

/// A marker not found, in a slot and in a point (the brain's `NOT_DETECTED`).
pub(crate) const NOT_DETECTED: f64 = -1.0;

/// The detector's classes in slot order: 0 Left, 1 Right, 2 Pivot, 3 End
/// (`AGENTS.md`, *Verified contracts*). Only Pivot and End enter the pool.
pub(crate) const CLASSES: usize = 4;
pub(crate) const PIVOT_CLASS: usize = 2;
pub(crate) const END_CLASS: usize = 3;

/// The command's three bytes: the sign, then the rate's size as a `u16`,
/// little-endian (the brain's `COMMAND_*`).
pub(crate) const COMMAND_SIZE: usize = 3;
pub(crate) const COMMAND_NEGATIVE: u8 = 0;
pub(crate) const COMMAND_POSITIVE: u8 = 1;

/// The GStreamer Data Protocol's packet header, as `gdppay` writes it (the
/// brain's `GDP_*`): 62 bytes, big-endian; byte 0 the major version, 1; bytes
/// 4–5 the payload's type; bytes 6–9 its length; bytes 10–17 the buffer's
/// timestamp in ns. A buffer, a frame, is type 1.
pub(crate) const GDP_HEADER_LENGTH: usize = 62;
pub(crate) const GDP_MAJOR_VERSION: u8 = 1;
pub(crate) const GDP_TYPE_OFFSET: usize = 4;
pub(crate) const GDP_LENGTH_OFFSET: usize = 6;
pub(crate) const GDP_TIMESTAMP_OFFSET: usize = 10;
pub(crate) const GDP_PAYLOAD_BUFFER: u16 = 1;

/// How long the camera software gets to shut down when asked, s, looked at
/// every `STOP_POLL_MS` (the brain's `CAMERA_STOP_TIMEOUT_S`: killed outright,
/// it can leave NVIDIA's camera service unable to open the camera again).
pub(crate) const CAMERA_STOP_TIMEOUT_S: u64 = 5;
pub(crate) const STOP_POLL_MS: u64 = 50;

/// How long the worker gets to end once its input pipe is closed, ms, before
/// it is killed (the brain's `WORKER_STOP_TIMEOUT_MS`).
pub(crate) const WORKER_STOP_TIMEOUT_MS: u64 = 1_000;

/// GStreamer's launcher, and how a process is asked to stop: the interrupt a
/// Ctrl-C sends.
pub(crate) const GST_LAUNCH: &str = "gst-launch-1.0";
pub(crate) const KILL: &str = "kill";
pub(crate) const KILL_INTERRUPT: &str = "-INT";

/// The camera pipeline's parts, as the brain's `pipeline.rs` puts them
/// together (each one explained in the brain's `constants.rs`).
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
pub(crate) const GST_ISP_GAIN_LOCKED: &str = "ispdigitalgainrange=1 1";
pub(crate) const GST_CAPS_NVMM: &str = "video/x-raw(memory:NVMM)";
pub(crate) const GST_CONVERTER: &str = "nvvidconv";
pub(crate) const GST_CAPS_RAW: &str = "video/x-raw";
pub(crate) const GST_FORMAT_GREY: &str = "GRAY8";
pub(crate) const GST_PAYLOADER: &str = "gdppay";
pub(crate) const GST_SINK: &str = "fdsink";
pub(crate) const GST_SINK_STDOUT: &str = "fd=1";

/// The socket the commands leave from: any of the Orin's addresses, and a port
/// the system picks (the brain's `ANY_ADDRESS`, `ANY_PORT`).
pub(crate) const ANY_ADDRESS: &str = "0.0.0.0";
pub(crate) const ANY_PORT: u16 = 0;

/// A little-endian `f64` on the worker's pipe, bytes.
pub(crate) const F64_BYTES: usize = 8;

/// The worker's two requests, the first byte of each (the brain's
/// `WORKER_FRAME_REQUEST`, `WORKER_POLICY_REQUEST`; `brain_worker.py`'s
/// `FRAME_REQUEST`, `POLICY_REQUEST`).
pub(crate) const WORKER_FRAME_REQUEST: u8 = b'F';
pub(crate) const WORKER_POLICY_REQUEST: u8 = b'P';

/// A little-endian `f32` on the worker's pipe, bytes.
pub(crate) const F32_BYTES: usize = 4;

/// A detection's five numbers, each a little-endian `f32`: centre x, centre y,
/// width, height, confidence (the brain's `DETECTION_FLOATS`,
/// `DETECTIONS_BYTES`). One detection is 20 bytes; the four classes' answer 80.
pub(crate) const DETECTION_FLOATS: usize = 5;
pub(crate) const DETECTION_BYTES: usize = DETECTION_FLOATS * F32_BYTES;
pub(crate) const DETECTIONS_BYTES: usize = CLASSES * DETECTION_BYTES;

/// The matrix `homography.json` stores, and the lens's camera matrix beside
/// it, are both 3 × 3 (`[x' y' w'] = H [u v 1]`; OpenCV's camera matrix).
pub(crate) const MATRIX_SIZE: usize = 3;

/// OpenCV's lens model takes 4, 5 or 8 bend coefficients, in the order k1, k2,
/// p1, p2, k3, k4, k5, k6; fewer than 8 means the rest are 0 (the brain's
/// `geometry/lens.rs`).
pub(crate) const LENS_COEFFICIENT_COUNTS: [usize; 3] = [4, 5, 8];
pub(crate) const LENS_COEFFICIENTS: usize = 8;

/// Undoing the lens's bend: repeated until the answer, bent back, lands within
/// this of the pixel asked about, or this many repeats have run (the brain's
/// `LENS_EPS_PX`, `LENS_ITERATIONS`).
pub(crate) const LENS_EPS_PX: f64 = 1e-12;
pub(crate) const LENS_ITERATIONS: usize = 100;

/// The names the program's two threads show in the system's process list.
pub(crate) const CAMERA_THREAD: &str = "camera";
pub(crate) const WORKER_READY_THREAD: &str = "worker-ready";

/// Nanoseconds in a millisecond.
pub(crate) const NANOS_PER_MILLI: f64 = 1_000_000.0;

/// A whole, in percent.
pub(crate) const PERCENT: f64 = 100.0;
