use serde::Deserialize;

/// Where the board should go next, in full-sensor pixels: the box every corner
/// must be inside, and the board's shape there.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct LensRect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
    /// The side turned away from the camera, as the picture shows it: `left`,
    /// `right`, `top` or `bottom`; empty when the board is held flat.
    pub(crate) tilt: String,
    /// The board as the camera sees it held there: top left, top right, bottom
    /// right, bottom left.
    pub(crate) shape: Vec<[f64; 2]>,
}
