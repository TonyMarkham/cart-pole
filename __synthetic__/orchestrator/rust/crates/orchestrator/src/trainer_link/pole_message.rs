use physics::PoleState;
use serde::Serialize;

/// 17: one capture's pole state in its pool entry, `{"speed": ω, "angle":
/// θ, "com": [x, y, z], "hinge": [x, y, z]}`: the pole joint's speed in
/// rad/s and angle in rad, the pole's centre of mass and the hinge point,
/// MuJoCo's world frame in meters (synthetic-loop-dia.md, Contracts, the
/// pole). Read at the capture's step with its pose and sites; for the
/// reward and the record alone.
#[derive(Serialize, Debug)]
pub struct PoleMessage {
    pub speed: f64,
    pub angle: f64,
    pub com: [f64; 3],
    pub hinge: [f64; 3],
}

impl From<PoleState> for PoleMessage {
    fn from(pole: PoleState) -> PoleMessage {
        PoleMessage {
            speed: pole.speed,
            angle: pole.angle,
            com: pole.com,
            hinge: pole.hinge,
        }
    }
}
