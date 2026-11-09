use crate::usb::command::Command;

/// What woke the board in the run loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunWake {
    /// A 2 ms tick: the ramp takes its next step.
    Tick,
    /// A command arrived, pulses/s: the ramp heads for it from the next tick.
    Command(i32),
    /// A line from the PC over USB.
    Usb(Command),
}
