/// One step's four timings, in whole timesteps from step 0's capture
/// (__documentation__/synthetic-loop-dia.md, *A step's four timings*): each the exact running
/// total rounded to the nearest timestep, halfway going later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepTimes {
    pub capture: i64,
    pub picture_ready: i64,
    pub send: i64,
    /// None: the command is lost.
    pub motor_change: Option<i64>,
}
