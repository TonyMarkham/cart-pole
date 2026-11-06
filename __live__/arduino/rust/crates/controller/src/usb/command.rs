use crate::constants::LINE_STATUS;

/// A line from the Pi's `arduino-manager` that the board acts on, in the
/// manager's protocol (its README, *The protocol*).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// `STATUS`: whether the board is alive, and its state. Answered
    /// `OK STATUS <state>`.
    Status,
}

impl Command {
    /// The command `line` is, or None for a line the board doesn't know.
    pub(crate) fn parse(line: &[u8]) -> Option<Self> {
        match line {
            LINE_STATUS => Some(Self::Status),
            _ => None,
        }
    }
}
