/// How a stretch of the Orin's commands ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MovingEnd {
    /// The cart came to rest: the command was 0, or the soft limit held it at
    /// a stop point. The board is CALIBRATED again.
    Rest,
    /// The count passed a stop point: the cart entered a protected end zone,
    /// and was stopped in one jump. The board is in FAULT.
    EndZone,
    /// A command with a bad sign byte arrived: the cart was stopped in one
    /// jump. The board is in FAULT.
    BadSign,
    /// A limit switch was pressed, or read pressed: the cart was stopped in
    /// one jump. The board is in FAULT.
    Switch,
}
