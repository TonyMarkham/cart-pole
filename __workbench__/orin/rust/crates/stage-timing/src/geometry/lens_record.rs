use serde::Deserialize;
use serde_json::Value;

/// The lens calibration as `homography.json` stores it beside the matrix
/// (`lens.py`, `to_json`): already halved for 728 × 544 frames.
/// `camera_matrix` is OpenCV's 3 × 3; `dist` its bend coefficients, written
/// flat by the lens solve, but taken nested too in case an older file has
/// OpenCV's row shape.
#[derive(Debug, Deserialize)]
pub(crate) struct LensRecord {
    pub(crate) camera_matrix: Vec<Vec<f64>>,
    pub(crate) dist: Value,
}
