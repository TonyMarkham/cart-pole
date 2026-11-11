use crate::{
    cart::{calibration_move::CalibrationMove, centre_speed::CentreSpeed, rail_end::RailEnd},
    constants::{
        CENTRE_REQUEST, CENTRE_SPEED_SEPARATOR, COMMAND_NEGATIVE, COMMAND_POSITIVE, COMMAND_SIZE,
        SEEK_IDLER_REQUEST, SEEK_MOTOR_REQUEST, STATE_REQUEST, STOP_REQUEST,
    },
    wifi::datagram::Datagram,
};

/// A datagram from the Orin that the board acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    /// `state`: what state the board is in (issue 7, step 1).
    State,
    /// A command: 3 bytes, the pulse rate to run at, pulses/s, its sign the
    /// direction.
    Command(i32),
    /// A command whose sign byte is neither 0 nor 1: a fault, by definition
    /// (the user, 2026-11-06, __documentation__/real-loop-dia.md, *The command*).
    BadSign,
    /// `CalibrateIdlerTravelLimit`, `CalibrateMotorTravelLimit`, or
    /// `MoveToCentre` with or without a speed: a move of the calibration.
    Calibrate(CalibrationMove),
    /// `Stop`: end the move under way. Outside a move it does nothing.
    Stop,
}

impl Message {
    /// What `datagram` asks, or None for one the board doesn't know. Its size
    /// decides first: exactly 3 bytes is a command.
    pub(crate) fn parse(datagram: &Datagram) -> Option<Self> {
        if datagram.size() == COMMAND_SIZE {
            return Self::command(datagram.payload());
        }
        match datagram.payload() {
            STATE_REQUEST => Some(Self::State),
            SEEK_IDLER_REQUEST => Some(Self::seek(RailEnd::Idler)),
            SEEK_MOTOR_REQUEST => Some(Self::seek(RailEnd::Motor)),
            STOP_REQUEST => Some(Self::Stop),
            payload => Self::centre(payload),
        }
    }

    /// The command in `payload`: byte 0 the sign (0 negative, 1 positive),
    /// bytes 1–2 the size, a little-endian u16 (__documentation__/real-loop-dia.md,
    /// *Contracts*). A size of 0 is rest. Any other sign byte is `BadSign`,
    /// whatever the size.
    fn command(payload: &[u8]) -> Option<Self> {
        let [sign, low, high] = *payload else {
            return None;
        };
        let size = i32::from(u16::from_le_bytes([low, high]));
        Some(match sign {
            COMMAND_NEGATIVE => Self::Command(-size),
            COMMAND_POSITIVE => Self::Command(size),
            _ => Self::BadSign,
        })
    }

    const fn seek(toward: RailEnd) -> Self {
        Self::Calibrate(CalibrationMove::Seek(toward))
    }

    /// `MoveToCentre`, or `MoveToCentre` and a space and a speed, or None for
    /// any other payload. A speed that isn't a whole number is still a move
    /// to the centre, answered `bad_speed`.
    fn centre(payload: &[u8]) -> Option<Self> {
        let speed = match payload.strip_prefix(CENTRE_REQUEST)? {
            [] => CentreSpeed::Default,
            [CENTRE_SPEED_SEPARATOR, digits @ ..] => CentreSpeed::parse(digits),
            _ => return None,
        };
        Some(Self::Calibrate(CalibrationMove::Centre(speed)))
    }
}
