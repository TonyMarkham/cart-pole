use serde::Serialize;

/// One joint's position in a state message: the MJCF joint name and its
/// `qpos` — meters for a slide, radians for a hinge (MuJoCo's runtime
/// units).
#[derive(Serialize, Debug)]
pub struct JointQpos {
    pub name: String,
    pub qpos: f64,
}
