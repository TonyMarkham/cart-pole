use crate::{
    constants::{FRAME_HEIGHT, FRAME_WIDTH, MATRIX_SIZE},
    geometry::{lens::Lens, matrix_record::MatrixRecord, point::Point},
};

use anyhow::{Context, Result, anyhow, bail};
use std::{fs, path::Path};

/// The stored matrix and the lens it was fitted with (the brain's
/// `geometry/homography.rs`). A pixel's metres: the bend undone, then
///
/// ```text
/// [x' y' w'] = H [u v 1]      x = x' / w'    z = y' / w'
/// ```
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Homography {
    matrix: [[f64; MATRIX_SIZE]; MATRIX_SIZE],
    lens: Lens,
}

impl Homography {
    /// Reads `homography.json`, and refuses one the brain would refuse: not
    /// ok, or fitted on frames of another size.
    pub(crate) fn load(path: &Path) -> Result<Homography> {
        let text = fs::read_to_string(path).with_context(|| {
            format!(
                "no matrix at {}: calibrate on /calibrate first",
                path.display()
            )
        })?;
        let record: MatrixRecord = serde_json::from_str(&text)
            .with_context(|| format!("{} could not be read", path.display()))?;
        if record.frame != [FRAME_WIDTH as u64, FRAME_HEIGHT as u64] {
            bail!(
                "{} was fitted on {:?} frames, and the brain's are {FRAME_WIDTH} x {FRAME_HEIGHT}",
                path.display(),
                record.frame
            );
        }
        if !(record.within_limit || record.accepted) {
            bail!(
                "the matrix in {} did not pass and was not accepted: calibrate again, or accept \
                 the fit on /calibrate",
                path.display()
            );
        }
        let mut matrix = [[0f64; MATRIX_SIZE]; MATRIX_SIZE];
        for (row, values) in matrix.iter_mut().enumerate() {
            for (column, value) in values.iter_mut().enumerate() {
                *value = record
                    .matrix
                    .get(row)
                    .and_then(|values| values.get(column))
                    .copied()
                    .ok_or_else(|| anyhow!("the matrix in {} is not 3 x 3", path.display()))?;
            }
        }
        Ok(Homography {
            matrix,
            lens: Lens::from_record(&record.lens)?,
        })
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
