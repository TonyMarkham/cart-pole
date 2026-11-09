/// What woke the board while it runs the Orin's commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MovingWake {
    /// A 2 ms tick: the ramp takes its next step.
    Tick,
    /// A command arrived, pulses/s: the ramp heads for it from the next tick.
    Command(i32),
    /// A command with a bad sign byte arrived: a fault.
    BadSign,
    /// A limit switch was pressed, or reads pressed: a fault.
    Switch,
}
