use crate::measure::{lens_used::LensUsed, residual::Residual};

use serde::Deserialize;
use std::collections::BTreeMap;

/// The fit's result: `fit` in `fit.py`, and where it was stored.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct FitReport {
    /// Every point within `limit_px` of where the matrix puts it.
    pub(crate) within_limit: bool,
    /// A person accepted it although a point is over the limit.
    pub(crate) accepted: bool,
    pub(crate) residuals: Vec<Residual>,
    pub(crate) worst_px: f64,
    pub(crate) limit_px: f64,
    /// The lens calibration whose bend was undone, and the worst point had it not been.
    pub(crate) lens: LensUsed,
    pub(crate) worst_uncorrected_px: f64,
    /// How long ago each stop was measured, by stop.
    pub(crate) kept_minutes_ago: BTreeMap<String, u64>,
    pub(crate) gaps_mm: BTreeMap<String, f64>,
    pub(crate) span_pulses: u32,
    pub(crate) span_m: f64,
    pub(crate) cad_length_m: f64,
    pub(crate) difference_m: f64,
    pub(crate) saved_to: Option<String>,
    /// This fit's own dated copy, never overwritten.
    pub(crate) dated_copy: Option<String>,
}
