use crate::{
    config::wifi_timing::FRAME_TIMEOUT_MS,
    wifi::{AT, at, datagram::Datagram, deadline::Deadline, frame_reader::FrameReader},
};

use unor4_hal::wake::{WakeError, WakeTimer};

/// The UDP socket the join opened, with push mode on: the bridge pushes each
/// datagram to the board. The controller's `wifi/udp_socket.rs` without its
/// answers: the bench never replies over WiFi, so its run loop does exactly
/// the controller's MOVING work on a datagram and no more.
pub(crate) struct UdpSocket {
    reader: FrameReader,
    /// When a frame whose header has ended must be whole by: its payload and
    /// line end within `frame_timeout_ms`, or it is dropped.
    frame_deadline: Option<Deadline>,
}

impl UdpSocket {
    pub(crate) const fn new() -> Self {
        Self {
            reader: FrameReader::new(),
            frame_deadline: None,
        }
    }

    /// Takes the bytes waiting from the chip, without sleeping, and returns a
    /// datagram once one is whole. A frame cut short is dropped once
    /// `frame_timeout_ms` has passed since its header, so it can't swallow
    /// the next.
    pub(crate) fn look(&mut self, wake: &mut WakeTimer) -> Result<Option<Datagram>, WakeError> {
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
