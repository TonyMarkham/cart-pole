/// A command sent and not yet landed or gobbled (__documentation__/synthetic-loop-dia.md,
/// *Commands in flight*).
#[derive(Clone, Copy, Debug)]
pub struct InFlight {
    /// Its motor change, in timesteps from step 0's capture.
    pub motor_change: i64,
    pub step: i64,
    pub command: i32,
}
