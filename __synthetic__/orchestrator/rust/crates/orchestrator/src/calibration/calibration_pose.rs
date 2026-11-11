/// One pose the calibration measures (__documentation__/synthetic-loop-dia.md, Calibration): the
/// two joints, and where MuJoCo says Pivot and End then are, each `(x, z)` in
/// metres in the pole's plane.
#[derive(Debug)]
pub struct CalibrationPose {
    pub name: &'static str,
    pub cart: f64,
    pub pole: f64,
    pub pivot: [f64; 2],
    pub end: [f64; 2],
}
