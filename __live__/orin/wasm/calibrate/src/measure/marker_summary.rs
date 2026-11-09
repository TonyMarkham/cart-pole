use serde::Deserialize;

/// One marker's measured point at a stop: `MarkerPoint` in `measure.py`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) struct MarkerSummary {
    pub(crate) u: f64,
    pub(crate) v: f64,
    pub(crate) spread: f64,
    pub(crate) dropped: u32,
    pub(crate) count: u32,
}
