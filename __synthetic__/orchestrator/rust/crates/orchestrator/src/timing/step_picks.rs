/// One step's five picks, in the order they are made
/// (synthetic-loop-dia.md, *The schedule*, *The order of the picks*), ms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepPicks {
    pub frame_ready: f64,
    pub to_uno: f64,
    pub change_motor: f64,
    pub next_frame_ready: f64,
    /// The drop came out 1: the command never has a motor change.
    pub lost: bool,
}
