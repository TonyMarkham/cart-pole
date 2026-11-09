use serde::Serialize;

/// The three places the cart can stand to be measured. The names are the ones
/// `calibrate_service.py` and the page use.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Stop {
    Motor,
    Idler,
    Centre,
}
