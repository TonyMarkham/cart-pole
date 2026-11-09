/// MJCF names as in the model's `<joint name>` / actuator element
/// `name` / `<site name>`. The joint names are also the wire keys of the
/// player's state message. The sites are named by role
/// (synthetic-loop-dia.md, Contracts, the sites).
#[derive(Debug, Clone)]
pub struct Names {
    pub cart_joint: String,
    pub pole_joint: String,
    pub actuator: String,
    pub left_site: String,
    pub right_site: String,
    pub pivot_site: String,
    pub end_site: String,
    pub target_site: String,
    pub dynamic_target_site: String,
    pub cart_site: String,
    pub cart_target_site: String,
    pub left_rail_site: String,
    pub left_cart_site: String,
    pub right_rail_site: String,
    pub right_cart_site: String,
}
