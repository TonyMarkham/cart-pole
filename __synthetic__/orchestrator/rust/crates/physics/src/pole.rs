/// The pole's state at one instant (synthetic-loop-dia.md, Contracts, the
/// pole): everything that swings on the pole joint, the body holding the
/// joint and every body under it. Every field is MuJoCo's own value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoleState {
    /// The pole joint's `qvel`, rad/s, signed about the joint's axis.
    pub speed: f64,
    /// The pole joint's `qpos`, rad: 0 hanging, π upright, counting on
    /// past a full turn.
    pub angle: f64,
    /// The pole's centre of mass, MuJoCo's `subtree_com` of the joint's
    /// body: a world point in the sites' frame, meters, z up.
    pub com: [f64; 3],
    /// The pole joint's anchor, MuJoCo's `xanchor`: the hinge point, in the
    /// same frame.
    pub hinge: [f64; 3],
}
