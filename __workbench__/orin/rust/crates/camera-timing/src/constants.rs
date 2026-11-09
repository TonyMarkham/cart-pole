//! Fixed facts. The camera's are copied from the brain's `constants.rs`
//! (`__live__/orin/rust/crates/brain`), so the camera here is the brain's.

/// The frame the brain reads: the camera's 1456 × 1088 halved on each side,
/// one byte per pixel (the brain's `FRAME_WIDTH`, `FRAME_HEIGHT`).
pub(crate) const FRAME_WIDTH: usize = 728;
pub(crate) const FRAME_HEIGHT: usize = 544;
pub(crate) const FRAME_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT;

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

/// The name this program gives the GPIO line while it holds it, which
/// `gpioinfo` shows as the line's consumer.
#[cfg(target_os = "linux")]
pub(crate) const LED_CONSUMER: &str = "camera-timing";

/// Conversions, by definition.
pub(crate) const NANOS_PER_MICRO: u64 = 1_000;
pub(crate) const NANOS_PER_MILLI: f64 = 1e6;
pub(crate) const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// A share as a percentage.
pub(crate) const PERCENT: f64 = 100.0;
