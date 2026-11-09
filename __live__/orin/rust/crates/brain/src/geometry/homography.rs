use crate::{
    constants::{FRAME_HEIGHT, FRAME_WIDTH},
    error::{app_error::AppError, app_result::AppResult},
    geometry::{lens::Lens, matrix_record::MatrixRecord, point::Point},
};

use std::{fs, path::Path};

/// The stored matrix and the lens it was fitted with, held in memory for the
/// whole run. A pixel's metres: the bend undone, then
///
/// ```text
/// [x' y' w'] = H [u v 1]      x = x' / w'    z = y' / w'
/// ```
///
/// (`calibrate-w-homography.md`, *Per frame*; `cv2.perspectiveTransform`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Homography {
    matrix: [[f64; 3]; 3],
    lens: Lens,
}

impl Homography {
    /// Reads `homography.json`, and refuses one the run may not use: not ok, or
    /// fitted on frames of another size. The words say what to do at the rig.
    pub(crate) fn load(path: &Path) -> AppResult<Homography> {
        let text = fs::read_to_string(path).map_err(|error| {
            AppError::homography(format!(
                "no matrix at {}: calibrate on /calibrate first ({error})",
                path.display()
            ))
        })?;
        let record: MatrixRecord = serde_json::from_str(&text).map_err(|error| {
            AppError::homography(format!("{} could not be read: {error}", path.display()))
        })?;
        Homography::from_record(&record, path)
    }

    /// The matrix from its record, with the checks `load` makes.
    pub(crate) fn from_record(record: &MatrixRecord, path: &Path) -> AppResult<Homography> {
        if record.frame != [FRAME_WIDTH as u64, FRAME_HEIGHT as u64] {
            return Err(AppError::homography(format!(
                "{} was fitted on {:?} frames, and the brain's are {FRAME_WIDTH} x {FRAME_HEIGHT}",
                path.display(),
                record.frame
            )));
        }
        if !(record.within_limit || record.accepted) {
            return Err(AppError::homography(format!(
                "the matrix in {} did not pass and was not accepted: calibrate again, or accept the fit \
                 on /calibrate",
                path.display()
            )));
        }
        let mut matrix = [[0f64; 3]; 3];
        for (row, values) in matrix.iter_mut().enumerate() {
            for (column, value) in values.iter_mut().enumerate() {
                *value = record
                    .matrix
                    .get(row)
                    .and_then(|values| values.get(column))
                    .copied()
                    .ok_or_else(|| {
                        AppError::homography(format!(
                            "the matrix in {} is not 3 x 3",
                            path.display()
                        ))
                    })?;
            }
        }
        Ok(Homography {
            matrix,
            lens: Lens::from_record(&record.lens)?,
        })
    }

    /// A matrix with a lens given directly, for the tests.
    #[cfg(test)]
    pub(crate) fn with_straight_lens(matrix: [[f64; 3]; 3], lens: Lens) -> Homography {
        Homography { matrix, lens }
    }

    /// The pixel `(u, v)` of the 728 × 544 frame as (x, z) in metres in the
    /// pole's plane.
    pub(crate) fn to_metres(&self, u: f64, v: f64) -> Point {
        let (u, v) = self.lens.undo(u, v);
        let h = &self.matrix;
        let x = h[0][0] * u + h[0][1] * v + h[0][2];
        let z = h[1][0] * u + h[1][1] * v + h[1][2];
        let w = h[2][0] * u + h[2][1] * v + h[2][2];
        Point { x: x / w, z: z / w }
    }
}
