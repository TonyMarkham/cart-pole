use crate::{
    constants::{COMMAND_SIZE, STATE_REQUEST},
    wifi::datagram::Datagram,
};

/// A datagram from the Orin that the board acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    /// `state`: what state the board is in (issue 7, step 1).
    State,
    /// A command: 3 bytes, the pulse rate to run at.
    Command,
}

impl Message {
    /// What `datagram` asks, or None for one the board doesn't know. Its size
    /// decides first: exactly 3 bytes is a command, whatever they hold.
    pub(crate) fn parse(datagram: &Datagram) -> Option<Self> {
        if datagram.size() == COMMAND_SIZE {
            return Some(Self::Command);
        }
        (datagram.payload() == STATE_REQUEST).then_some(Self::State)
    }
}
