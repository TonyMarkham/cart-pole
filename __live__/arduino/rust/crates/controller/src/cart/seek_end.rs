/// How a seek ended, told from the end it was sent to. Each is one answer to
/// the Orin (`Answer::seek`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SeekEnd {
    /// That end's switch tripped: the seek reached its end.
    Arrived,
    /// The other end's switch tripped, or still read pressed once the cart
    /// should have left it: the cart went the wrong way.
    OtherEnd,
    /// That end's switch read pressed at the start: the cart didn't move.
    AlreadyThere,
    /// Both switches read pressed at the start: the cart didn't move, since
    /// the cart can't be at both ends, so a wire or a switch is at fault.
    Refused,
    /// A `Stop` arrived during the seek.
    Stopped,
    /// No switch tripped within `seek_timeout_ms`.
    TimedOut,
}
