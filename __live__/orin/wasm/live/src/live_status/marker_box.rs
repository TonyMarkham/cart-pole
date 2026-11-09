use serde::Deserialize;

/// A marker's box in a frame, in pixels.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) struct MarkerBox {
    pub(crate) cx: f64,
    pub(crate) cy: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}
