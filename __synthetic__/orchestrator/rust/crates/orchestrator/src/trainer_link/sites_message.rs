use physics::Sites;
use serde::Serialize;

/// 17: one capture's twelve true site positions in its pool entry,
/// `{"left": [x, y, z], "right": […], "pivot": […], "end": […], "target":
/// […], "dynamic_target": […], "cart": […], "cart_target": […],
/// "left_rail": […], "left_cart": […], "right_rail": […],
/// "right_cart": […]}`, MuJoCo's world frame in meters
/// (__documentation__/synthetic-loop-dia.md, Contracts, the sites). Read at the capture's
/// step with its pose, so they show the instant the frame shows; for the
/// reward alone.
#[derive(Serialize, Debug)]
pub struct SitesMessage {
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

impl From<Sites> for SitesMessage {
    fn from(sites: Sites) -> SitesMessage {
        SitesMessage {
            left: sites.left,
            right: sites.right,
            pivot: sites.pivot,
            end: sites.end,
            target: sites.target,
            dynamic_target: sites.dynamic_target,
            cart: sites.cart,
            cart_target: sites.cart_target,
            left_rail: sites.left_rail,
            left_cart: sites.left_cart,
            right_rail: sites.right_rail,
            right_cart: sites.right_cart,
        }
    }
}
