//! From the detector's pixels to metres in the pole's plane: the lens's bend
//! undone, then the matrix the calibration fitted (`real-loop-dia.md`,
//! *Contracts*, the points).

pub(crate) mod homography;
pub(crate) mod lens;
pub(crate) mod lens_record;
pub(crate) mod matrix_record;
pub(crate) mod point;
