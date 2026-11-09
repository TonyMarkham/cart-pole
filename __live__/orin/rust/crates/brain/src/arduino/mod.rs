//! The brain's link to the rig's Arduino: one UDP socket, the command datagrams
//! out, and whatever the board sends back read after each one.

pub(crate) mod board_reply;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    arduino::board_reply::BoardReply,
    command::Command,
    constants::{ANY_ADDRESS, ANY_PORT, REPLY_MAX},
    error::app_result::AppResult,
    settings::arduino_settings::ArduinoSettings,
};

use std::{io, net::UdpSocket};

/// A socket connected to the board for the length of a run. It never waits for
/// an answer: a command gets none outside FAULT and UNCALIBRATED, and the
/// board's unasked FAULT STATE message comes to whoever sent the latest
/// datagram, so it lands here.
pub(crate) struct Link {
    socket: UdpSocket,
}

impl Link {
    pub(crate) fn connect(arduino: &ArduinoSettings) -> AppResult<Link> {
        let socket = UdpSocket::bind((ANY_ADDRESS, ANY_PORT))?;
        socket.connect((arduino.address.as_str(), arduino.port))?;
        socket.set_nonblocking(true)?;
        Ok(Link { socket })
    }

    /// One command, one datagram.
    pub(crate) fn send(&self, command: Command) -> AppResult<()> {
        self.socket.send(&command.to_bytes())?;
        Ok(())
    }

    /// Every datagram the board has sent since the last look, those that are
    /// replies. Nothing waits: an empty socket gives an empty list.
    pub(crate) fn replies(&self) -> Vec<BoardReply> {
        let mut replies = Vec::new();
        let mut buffer = [0u8; REPLY_MAX];
        loop {
            match self.socket.recv(&mut buffer) {
                Ok(length) => {
                    if let Some(reply) = BoardReply::parse(&buffer[..length]) {
                        replies.push(reply);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                // A connected UDP socket reports an unreachable board as an
                // error on the next receive; the run goes on, since a lost
                // datagram is the WiFi's and the next command follows.
                Err(_) => break,
            }
        }
        replies
    }
}
