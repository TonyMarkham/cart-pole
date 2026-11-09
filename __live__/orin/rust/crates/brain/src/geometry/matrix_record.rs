use crate::geometry::lens_record::LensRecord;

use serde::Deserialize;

/// `homography.json` as `calibrate_service.py` writes it (`fit.py`, `store`),
/// the keys the brain reads: the matrix, the lens it was fitted with, whether
/// every point was within the limit, whether a person accepted a fit that was
/// not, and the frame size the pixels refer to.
#[derive(Debug, Deserialize)]
pub(crate) struct MatrixRecord {
    pub(crate) matrix: Vec<Vec<f64>>,
    pub(crate) lens: LensRecord,
    #[serde(default)]
    pub(crate) within_limit: bool,
    #[serde(default)]
    pub(crate) accepted: bool,
    pub(crate) frame: Vec<u64>,
}
