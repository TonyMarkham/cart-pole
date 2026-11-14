use serde::Serialize;

/// Three numbers written as Unity's `Vector3` in JSON, `{"x", "y", "z"}`,
/// the form the player's StateServer reads.
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Xyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
