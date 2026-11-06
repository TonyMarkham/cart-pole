use crate::{
    cart::rail_end::RailEnd,
    constants::{
        COMMAND_SIZE, SEEK_IDLER_REQUEST, SEEK_MOTOR_REQUEST, STATE_REQUEST, STOP_REQUEST,
    },
    wifi::datagram::Datagram,
};

/// A datagram from the Orin that the board acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    /// `state`: what state the board is in (issue 7, step 1).
    State,
    /// A command: 3 bytes, the pulse rate to run at.
    Command,
    /// `CalibrateIdlerTravelLimit` or `CalibrateMotorTravelLimit`: seek that
    /// end of the rail.
    Seek(RailEnd),
    /// `Stop`: end the move under way. Outside a move it does nothing.
    Stop,
}

impl Message {
    /// What `datagram` asks, or None for one the board doesn't know. Its size
    /// decides first: exactly 3 bytes is a command, whatever they hold.
    pub(crate) fn parse(datagram: &Datagram) -> Option<Self> {
        if datagram.size() == COMMAND_SIZE {
            return Some(Self::Command);
        }
        match datagram.payload() {
            STATE_REQUEST => Some(Self::State),
            SEEK_IDLER_REQUEST => Some(Self::Seek(RailEnd::Idler)),
            SEEK_MOTOR_REQUEST => Some(Self::Seek(RailEnd::Motor)),
            STOP_REQUEST => Some(Self::Stop),
            _ => None,
        }
    }
}
