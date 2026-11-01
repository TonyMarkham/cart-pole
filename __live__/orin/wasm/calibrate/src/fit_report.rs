use crate::residual::Residual;

use serde::Deserialize;
use std::collections::BTreeMap;

/// The fit's result: `fit` in `fit.py`, and where it was stored.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct FitReport {
    pub(crate) ok: bool,
    pub(crate) residuals: Vec<Residual>,
    pub(crate) worst_px: f64,
    pub(crate) limit_px: f64,
    /// The lens number this fit found, and its worst point with no lens correction.
    pub(crate) lens_k: f64,
    pub(crate) worst_uncorrected_px: f64,
    /// How long ago each stop was measured, by stop.
    pub(crate) kept_minutes_ago: BTreeMap<String, u64>,
    pub(crate) gaps_mm: BTreeMap<String, f64>,
    pub(crate) span_pulses: u32,
    pub(crate) span_m: f64,
    pub(crate) cad_length_m: f64,
    pub(crate) difference_m: f64,
    pub(crate) saved_to: Option<String>,
}
