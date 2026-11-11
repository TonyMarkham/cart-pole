use serde::Serialize;

/// The run's values, from the model MuJoCo loaded and the drive figures:
/// the cart top speed in m/s (the actuator's ctrlrange), the top pulse
/// rate in pulses/s (the synthetic Arduino's clamp), the timestep in s,
/// and the pole's body (__documentation__/synthetic-loop-dia.md, Contracts, the pole): its
/// mass in kg, its centre of mass's distance from the hinge axis in m, its
/// inertia about that axis in kg·m², and gravity in m/s².
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub struct RunValues {
    pub cart_top_speed: f64,
    pub top_pulse_rate: i32,
    pub timestep: f64,
    pub pole_mass: f64,
    pub pole_com_distance: f64,
    pub pole_inertia: f64,
    pub gravity: f64,
}
