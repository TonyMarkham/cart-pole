use serde::Deserialize;

/// `[camera]`: the sensor's mode, shutter and gain, the same values as the
/// Python's `[camera]` and `[stream]`, because the matrix was fitted on the
/// frames those made. `settings.toml` explains each where it is set.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CameraSettings {
    pub(crate) sensor_id: u32,
    pub(crate) sensor_mode: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) framerate: u32,
    pub(crate) tnr_mode: u32,
    pub(crate) ee_mode: u32,
    pub(crate) exposure_ns: u64,
    pub(crate) gain: u32,
    pub(crate) settle_frames: u64,
}
