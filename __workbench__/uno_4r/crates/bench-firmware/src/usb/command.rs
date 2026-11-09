use crate::constants::LINE_STATUS;

/// A line from the PC's bench-host that the board acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// `STATUS`: whether the board is running, and its address. Answered
    /// `OK STATUS running <address> <port>`, or `OK STATUS stopped <why>`.
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
