/// One progress bar's reading: how full it is, from 0 to 1; what it says; whether
/// it has reached what it needs, which draws it green; and where, from 0 to 1, a
/// tick marks the least that counts, if it has one.
#[derive(Debug, PartialEq)]
pub(crate) struct Meter {
    pub(crate) fraction: f64,
    pub(crate) label: String,
    pub(crate) good: bool,
    pub(crate) mark: Option<f64>,
}
