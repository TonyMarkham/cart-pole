use crate::{
    constants::{LENS_EPS_PX, LENS_ITERATIONS},
    error::app_error::AppError,
    geometry::lens_record::LensRecord,
};

use serde_json::Value;

/// The lens's bend, and how to undo it, in the pixels of the 728 × 544 frame.
/// The lens bows straight lines outward from the picture's middle, which the
/// matrix cannot follow, so every pixel is first moved to where a lens with no
/// bend would have put it (`lens.py`; `__documentation__/calibrate-w-homography.md`, *The
/// lens*). OpenCV's model: a pixel is taken to the normalised plane,
/// `x = (u - cx) / fx`, `y = (v - cy) / fy`, and there the bend is
///
/// ```text
/// r2 = x² + y²
/// radial = (1 + k1 r2 + k2 r2² + k3 r2³) / (1 + k4 r2 + k5 r2² + k6 r2³)
/// x_bent = x radial + 2 p1 x y + p2 (r2 + 2 x²)
/// y_bent = y radial + p1 (r2 + 2 y²) + 2 p2 x y
/// ```
///
/// with the coefficients in OpenCV's order k1, k2, p1, p2, k3, k4, k5, k6; the
/// lens solve gives the first five, and the rest are 0. Undoing it has no
/// formula: `undo` repeats `x = (x_bent - tangential) / radial` from the bent
/// point until bending the answer back lands within `LENS_EPS_PX` of the
/// pixel asked about, as OpenCV's `undistortPoints` does with `lens.py`'s
/// stopping point, so the brain's points match the fit's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lens {
    fx: f64,
    fy: f64,
    cx: f64,
    cy: f64,
    k: [f64; 8],
}

impl Lens {
    /// The lens from its record in `homography.json`.
    pub(crate) fn from_record(record: &LensRecord) -> Result<Lens, AppError> {
        let at = |row: usize, column: usize| -> Result<f64, AppError> {
            record
                .camera_matrix
                .get(row)
                .and_then(|values| values.get(column))
                .copied()
                .ok_or_else(|| AppError::homography("the lens's camera_matrix is not 3 x 3"))
        };
        let fx = at(0, 0)?;
        let fy = at(1, 1)?;
        if fx == 0.0 || fy == 0.0 {
            return Err(AppError::homography("the lens's focal lengths are 0"));
        }
        let coefficients = flatten(&record.dist);
        if !matches!(coefficients.len(), 4 | 5 | 8) {
            return Err(AppError::homography(format!(
                "the lens has {} bend coefficients; OpenCV's model has 4, 5 or 8",
                coefficients.len()
            )));
        }
        let mut k = [0f64; 8];
        k[..coefficients.len()].copy_from_slice(&coefficients);
        Ok(Lens {
            fx,
            fy,
            cx: at(0, 2)?,
            cy: at(1, 2)?,
            k,
        })
    }

    /// A lens with no bend and these focal lengths and centre, for the tests.
    #[cfg(test)]
    pub(crate) fn straight(fx: f64, fy: f64, cx: f64, cy: f64) -> Lens {
        Lens {
            fx,
            fy,
            cx,
            cy,
            k: [0.0; 8],
        }
    }

    /// Where a lens with no bend would have put the pixel `(u, v)`.
    pub(crate) fn undo(&self, u: f64, v: f64) -> (f64, f64) {
        let x0 = (u - self.cx) / self.fx;
        let y0 = (v - self.cy) / self.fy;
        let (mut x, mut y) = (x0, y0);
        for _ in 0..LENS_ITERATIONS {
            let (radial, tangential_x, tangential_y) = self.bend_parts(x, y);
            if radial <= 0.0 {
                // Past the model's reach: OpenCV gives the point back as it was.
                x = x0;
                y = y0;
                break;
            }
            x = (x0 - tangential_x) / radial;
            y = (y0 - tangential_y) / radial;
            let (bent_u, bent_v) = self.bend(x, y);
            if ((bent_u - u).powi(2) + (bent_v - v).powi(2)).sqrt() < LENS_EPS_PX {
                break;
            }
        }
        (self.fx * x + self.cx, self.fy * y + self.cy)
    }

    /// The pixel this lens shows a normalised point at: the bend applied.
    pub(crate) fn bend(&self, x: f64, y: f64) -> (f64, f64) {
        let (radial, tangential_x, tangential_y) = self.bend_parts(x, y);
        (
            self.fx * (x * radial + tangential_x) + self.cx,
            self.fy * (y * radial + tangential_y) + self.cy,
        )
    }

    /// The radial factor and the two tangential shifts at a normalised point.
    fn bend_parts(&self, x: f64, y: f64) -> (f64, f64, f64) {
        let [k1, k2, p1, p2, k3, k4, k5, k6] = self.k;
        let r2 = x * x + y * y;
        let radial =
            (1.0 + r2 * (k1 + r2 * (k2 + r2 * k3))) / (1.0 + r2 * (k4 + r2 * (k5 + r2 * k6)));
        let tangential_x = 2.0 * p1 * x * y + p2 * (r2 + 2.0 * x * x);
        let tangential_y = p1 * (r2 + 2.0 * y * y) + 2.0 * p2 * x * y;
        (radial, tangential_x, tangential_y)
    }
}

/// The numbers in a JSON array, nested to any depth, in order.
fn flatten(value: &Value) -> Vec<f64> {
    match value {
        Value::Array(items) => items.iter().flat_map(flatten).collect(),
        Value::Number(number) => number.as_f64().into_iter().collect(),
        _ => Vec::new(),
    }
}
