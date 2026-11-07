use serde::Deserialize;

/// The lens calibration whose bend the fit undid first: `Lens.to_json` in
/// `lens.py`. Only what the page shows.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct LensUsed {
    /// When the lens was solved, on the /lens page.
    pub(crate) date: String,
    /// That solve's average error, pixels.
    pub(crate) rms_px: f64,
}
