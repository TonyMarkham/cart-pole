use serde::Deserialize;
use std::fmt;

/// The five numbers DeepMind's cart-pole policy reads, as the synthetic Orin
/// worked them out from the markers it detected in this frame
/// (`__synthetic__/orin/python/src/derived_state.py`). Each is `None` where a
/// marker it needs was not seen.
///
/// These are what the camera says, so they carry its error and its delay.
/// MuJoCo's true values are the capture's, never these.
#[derive(Deserialize, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DerivedState {
    /// The cart's position along the rail, m from the centre stop.
    pub cart: Option<f64>,
    /// How much of the pole points up: 1 straight up, -1 hanging.
    pub cos_theta: Option<f64>,
    /// How much of the pole leans along the rail, positive towards +x.
    pub sin_theta: Option<f64>,
    /// How fast the cart moves, m/s, positive towards +x.
    pub cart_speed: Option<f64>,
    /// How fast the pole turns, rad/s, positive from straight up towards +x:
    /// the opposite sign to MuJoCo's qvel for this rig's pole joint.
    pub pole_speed: Option<f64>,
}

/// One number for the log line, or `none` where the marker was not seen.
pub(crate) fn number(value: Option<f64>) -> String {
    value.map_or("none".to_string(), |v| format!("{v:+.3}"))
}

impl fmt::Display for DerivedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cart {} m, cos {}, sin {}, cart speed {} m/s, pole speed {} rad/s",
            number(self.cart),
            number(self.cos_theta),
            number(self.sin_theta),
            number(self.cart_speed),
            number(self.pole_speed),
        )
    }
}
