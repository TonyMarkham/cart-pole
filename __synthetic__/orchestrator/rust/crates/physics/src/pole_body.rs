/// What the pole's energy needs and the pose never changes, read once at
/// load from the model (__documentation__/synthetic-loop-dia.md, Contracts, the run's values).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoleBody {
    /// kg: `body_subtreemass` of the pole joint's body.
    pub mass: f64,
    /// m: the centre of mass's distance from the joint's axis.
    pub com_distance: f64,
    /// kg·m²: the moment of inertia about the joint's axis, the joint's
    /// diagonal entry in MuJoCo's inertia matrix (armature included).
    pub inertia: f64,
    /// m/s²: the size of the model's gravity, which load proved points
    /// straight down (−z).
    pub gravity: f64,
}
