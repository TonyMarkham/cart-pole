//! From the detector's pixels to metres in the pole's plane, as the brain
//! does it (the brain's `geometry/`): the lens's bend undone, then the matrix
//! the calibration fitted.

pub(crate) mod homography;
pub(crate) mod lens;
pub(crate) mod lens_record;
pub(crate) mod matrix_record;
pub(crate) mod point;
