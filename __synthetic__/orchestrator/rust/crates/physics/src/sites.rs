/// The twelve sites' world positions for the current `qpos`, by role
/// (__documentation__/synthetic-loop-dia.md, Contracts, the sites), in MuJoCo's world frame:
/// meters, z up (Unity's (x, y, z) is MuJoCo's (x, z, y)). Left, Right,
/// Pivot and End are the markers' centres; Target is fixed where End sits
/// with the cart centred and the pole upright; Dynamic Target rides the
/// cart where End sits with the pole upright; Cart rides the cart at the
/// slider joint, and Cart Target is fixed where Cart sits with the cart
/// centred. The strike faces: each rail
/// site is where the cart would strike that base, each cart site the
/// cart's face that would strike it, so `left_cart.x - left_rail.x` and
/// `right_rail.x - right_cart.x` are the clearances, 0 at contact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sites {
    pub left: [f64; 3],
    pub right: [f64; 3],
    pub pivot: [f64; 3],
    pub end: [f64; 3],
    pub target: [f64; 3],
    pub dynamic_target: [f64; 3],
    pub cart: [f64; 3],
    pub cart_target: [f64; 3],
    pub left_rail: [f64; 3],
    pub left_cart: [f64; 3],
    pub right_rail: [f64; 3],
    pub right_cart: [f64; 3],
}
