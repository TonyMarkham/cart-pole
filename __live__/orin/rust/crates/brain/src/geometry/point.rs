use crate::constants::NOT_DETECTED;

/// A place in the pole's plane, metres: `x` along the rail, positive to the
/// right as the camera sees it, `z` up (`calibrate-w-homography.md`, *Terms*).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) z: f64,
}

impl Point {
    /// A marker not found: there is no pixel to map (`real-loop-dia.md`,
    /// *Contracts*, the points).
    pub(crate) const NOT_DETECTED: Point = Point {
        x: NOT_DETECTED,
        z: NOT_DETECTED,
    };

    /// The two numbers for the log and the pool.
    pub(crate) fn as_array(self) -> [f64; 2] {
        [self.x, self.z]
    }
}
