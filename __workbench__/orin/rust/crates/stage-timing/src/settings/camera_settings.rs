use serde::Deserialize;

/// `[camera]`: the brain's camera values (its `settings/camera_settings.rs`),
/// each explained in `__live__/orin/rust/settings.toml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CameraSettings {
    /// Which camera, and which of its modes.
    pub(crate) sensor_id: u32,
    pub(crate) sensor_mode: u32,
    /// The sensor's frame, pixels, and its rate, frames/s.
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) framerate: u32,
    /// Noise reduction and edge sharpening, 0 = off.
    pub(crate) tnr_mode: u32,
    pub(crate) ee_mode: u32,
    /// The shutter, ns, locked.
    pub(crate) exposure_ns: u64,
    /// The sensor's gain, tenths of a dB, locked.
    pub(crate) gain: u32,
    /// Frames thrown away after the camera starts, while it settles.
    pub(crate) settle_frames: u64,
}
