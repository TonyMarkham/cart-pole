use std::f64::consts::TAU;

use physics::Pose;

/// State `i` of an `n`-state sweep, the renderer's smoke test: the cart
/// makes one sine cycle at 0.4 m amplitude — inside the MJCF's ±0.4 m
/// `ctrlrange` — while the pole makes one full turn starting from
/// hanging, so every pole angle is rendered once.
pub fn sweep(i: u32, n: u32) -> Pose {
    let phase = f64::from(i) / f64::from(n.max(1));
    Pose {
        cart: 0.4 * (TAU * phase).sin(),
        pole: TAU * phase,
    }
}
