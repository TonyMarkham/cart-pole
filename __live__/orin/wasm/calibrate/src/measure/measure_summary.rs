use crate::measure::marker_summary::MarkerSummary;

use serde::Deserialize;

/// How one stop's measurement went.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) struct MeasureSummary {
    /// The hand-measured gap at this stop; the centre has none.
    pub(crate) gap_mm: Option<f64>,
    pub(crate) images: u32,
    pub(crate) seen: u32,
    pub(crate) took_s: f64,
    pub(crate) pivot: MarkerSummary,
    pub(crate) end: MarkerSummary,
}
