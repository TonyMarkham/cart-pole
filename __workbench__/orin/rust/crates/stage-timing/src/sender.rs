use crate::{
    command::Command,
    constants::{ANY_ADDRESS, ANY_PORT},
    settings::send_settings::SendSettings,
};

use anyhow::{Context, Result};
use std::net::UdpSocket;

/// The command datagrams out, from one UDP socket, as the brain's
/// `arduino/mod.rs` `Link` sends them. They go to `[send]`'s address, not the
/// rig's Arduino, so the cart never moves during the test; the board's
/// replies, which the brain reads after each send, are not read here.
pub(crate) struct Sender {
    socket: UdpSocket,
}

impl Sender {
    pub(crate) fn connect(send: &SendSettings) -> Result<Sender> {
        let socket = UdpSocket::bind((ANY_ADDRESS, ANY_PORT)).context("binding the socket")?;
        socket
            .connect((send.address.as_str(), send.port))
            .with_context(|| format!("aiming the socket at {}:{}", send.address, send.port))?;
        socket
            .set_nonblocking(true)
            .context("making the socket non-blocking")?;
        Ok(Sender { socket })
    }

    /// One command, one datagram. It returns once the system has the datagram
    /// to send: the moment the brain's log calls `sent`.
    pub(crate) fn send(&self, command: Command) -> Result<()> {
        self.socket
            .send(&command.to_bytes())
            .context("sending the datagram")?;
        Ok(())
    }
}
