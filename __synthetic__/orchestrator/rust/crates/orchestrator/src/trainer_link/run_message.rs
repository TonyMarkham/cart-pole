use crate::trainer_link::run_values::RunValues;

use serde::Serialize;

/// The answer, the orchestrator's first stdout line:
/// `{"run": {"cart_top_speed": v, "top_pulse_rate": r, "timestep": s,
/// "pole_mass": m, "pole_com_distance": l, "pole_inertia": I, "gravity":
/// g}}`.
#[derive(Serialize)]
pub struct RunMessage<'a> {
    pub run: &'a RunValues,
}
