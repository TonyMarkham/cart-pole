/// One cart-pole state: cart position in meters along the slide, pole
/// angle in radians about the hinge (0 = hanging, π = upright) —
/// MuJoCo's `qpos` for the two joints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub cart: f64,
    pub pole: f64,
}
