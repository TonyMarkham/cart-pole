use serde::Deserialize;

/// A solve: OpenCV's `calibrateCamera` on the kept stills (`lens_calibration.solve`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct LensResult {
    /// How far, on average, the board's corners are from where the result puts them.
    pub(crate) rms_px: f64,
    /// fx 0 cx / 0 fy cy / 0 0 1, in full-sensor pixels.
    pub(crate) camera_matrix: Vec<Vec<f64>>,
    /// k1, k2, p1, p2, k3.
    pub(crate) dist: Vec<f64>,
    pub(crate) monotonic: bool,
    pub(crate) views: u32,
    pub(crate) saved_to: String,
}
