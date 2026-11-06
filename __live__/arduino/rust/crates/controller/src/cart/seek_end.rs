/// How a seek of the idler's end ended. Each is one answer to the Orin
/// (`Answer::seek_idler`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SeekEnd {
    /// The idler's switch tripped: the seek reached its end.
    Idler,
    /// The motor's switch tripped, or still read pressed once the cart should
    /// have left it: the cart went the other way, towards the motor's end.
    Motor,
    /// The idler's switch read pressed at the start: the cart didn't move.
    AlreadyThere,
    /// Both switches read pressed at the start: the cart didn't move, since
    /// the cart can't be at both ends, so a wire or a switch is at fault.
    Refused,
    /// A `Stop` arrived during the seek.
    Stopped,
    /// No switch tripped within `seek_timeout_ms`.
    TimedOut,
}
