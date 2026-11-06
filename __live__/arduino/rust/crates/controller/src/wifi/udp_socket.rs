use crate::{
    config::wifi_timing::{FRAME_TIMEOUT_MS, REPLY_TIMEOUT_MS},
    constants::{PROMPT_UDPREMOTEIP, PROMPT_UDPREMOTEPORT},
    wifi::{
        AT, asker::Asker, at, datagram::Datagram, deadline::Deadline, frame_reader::FrameReader,
        next_byte, reply::Reply,
    },
};

use core::fmt;
use unor4_hal::wake::{WakeError, WakeTimer};

/// The UDP socket the join opened, with push mode on: the bridge pushes each
/// datagram to the board, and the board answers whoever sent the latest one.
pub(crate) struct UdpSocket {
    /// The socket's number, from `+UDPBEGIN`.
    number: i32,
    reader: FrameReader,
    /// A datagram that arrived while the board waited for an AT reply, kept
    /// for the next `next_datagram`. Only the latest is kept.
    waiting: Option<Datagram>,
    /// When a frame whose header has ended must be whole by: its payload and
    /// line end within `frame_timeout_ms`, or it is dropped.
    frame_deadline: Option<Deadline>,
}

impl UdpSocket {
    pub(crate) const fn new(number: i32) -> Self {
        Self {
            number,
            reader: FrameReader::new(),
            waiting: None,
            frame_deadline: None,
        }
    }

    /// Who sent the latest datagram, or None if either exchange was refused or
    /// timed out.
    pub(crate) fn asker(&mut self, wake: &mut WakeTimer) -> Result<Option<Asker>, WakeError> {
        let socket = self.number;

        let Some(address) = self
            .ask(wake, format_args!("AT+UDPREMOTEIP={socket}\r\n"), &[])?
            .filter(Reply::ok)
        else {
            return Ok(None);
        };
        let Some(port) = self
            .ask(wake, format_args!("AT+UDPREMOTEPORT={socket}\r\n"), &[])?
            .filter(Reply::ok)
        else {
            return Ok(None);
        };
        let (Some(address), Some(port)) = (
            address.text_after(PROMPT_UDPREMOTEIP),
            port.number_after(PROMPT_UDPREMOTEPORT),
        ) else {
            return Ok(None);
        };
        Ok(Asker::new(address, port))
    }

    /// Sends `payload` to `asker`. False if any of the three exchanges was
    /// refused or timed out: the answer is dropped, and the asker's wait for it
    /// times out.
    pub(crate) fn send_to(
        &mut self,
        wake: &mut WakeTimer,
        asker: &Asker,
        payload: &[u8],
    ) -> Result<bool, WakeError> {
        let socket = self.number;
        let (address, port) = (asker.address(), asker.port());

        let begun = self.ask(
            wake,
            format_args!("AT+UDPBEGINPACKETIP={socket},{port},{address}\r\n"),
            &[],
        )?;
        if !begun.is_some_and(|reply| reply.ok()) {
            return Ok(false);
        }

        let written = self.ask(
            wake,
            format_args!("AT+UDPWRITE={socket},{}\r\n", payload.len()),
            payload,
        )?;
        if !written.is_some_and(|reply| reply.ok()) {
            return Ok(false);
        }

        let ended = self.ask(wake, format_args!("AT+UDPENDPACKET={socket}\r\n"), &[])?;
        Ok(ended.is_some_and(|reply| reply.ok()))
    }

    /// With push mode on: sends `command`, then `data`, and collects the reply.
    /// No drain first, since that would throw away a datagram. The bytes of a
    /// datagram arriving meanwhile go to the reader, not the reply, and the
    /// datagram is kept for `next_datagram`.
    fn ask(
        &mut self,
        wake: &mut WakeTimer,
        command: fmt::Arguments,
        data: &[u8],
    ) -> Result<Option<Reply>, WakeError> {
        // Writing to the link never fails (`fmt::Write for &UartIrq`).
        let _ = fmt::Write::write_fmt(&mut &AT, command);
        AT.write_all(data);

        let mut deadline = Deadline::start(wake, at::millis(REPLY_TIMEOUT_MS))?;
        let mut reply = Reply::new();
        while let Some(byte) = next_byte(wake, &mut deadline)? {
            let was_in_frame = self.reader.in_frame();
            if let Some(datagram) = self.reader.push(byte) {
                self.waiting = Some(datagram);
                reply.drop_line();
                continue;
            }
            if was_in_frame || self.reader.in_frame() {
                continue;
            }
            if reply.push(byte) {
                return Ok(Some(reply));
            }
        }
        // No reply in time. A frame left half-read would swallow the next
        // reply's bytes, so it is dropped.
        if self.reader.in_frame() {
            self.reader.reset();
        }
        Ok(None)
    }

    /// Takes the bytes waiting from the chip, without sleeping, and returns a
    /// datagram once one is whole. One kept during an answer comes first. A
    /// frame cut short is dropped once `frame_timeout_ms` has passed since its
    /// header, so it can't swallow the next.
    pub(crate) fn look(&mut self, wake: &mut WakeTimer) -> Result<Option<Datagram>, WakeError> {
        if let Some(datagram) = self.waiting.take() {
            return Ok(Some(datagram));
        }
        while let Some(byte) = AT.read_byte() {
            let was_in_frame = self.reader.in_frame();
            if let Some(datagram) = self.reader.push(byte) {
                self.frame_deadline = None;
                return Ok(Some(datagram));
            }
            if !was_in_frame && self.reader.in_frame() {
                self.frame_deadline = Some(Deadline::start(wake, at::millis(FRAME_TIMEOUT_MS))?);
            }
        }
        if let Some(deadline) = self.frame_deadline.as_mut()
            && deadline.passed(wake)?
        {
            self.reader.reset();
            self.frame_deadline = None;
        }
        Ok(None)
    }
}
