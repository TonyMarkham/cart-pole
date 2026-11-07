use serde::Deserialize;

/// How far one measured point is from where the matrix puts it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct Residual {
    pub(crate) point: String,
    pub(crate) px: f64,
}
