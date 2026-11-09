use crate::{
    clock::Clock,
    config::wifi::UDP_PORT,
    constants::{
        FIELD_SEPARATOR, LINE_END, PING_PREFIX, PING_SIZE, REPLY_STATUS, STATUS_RUNNING,
        SYNC_PREFIX,
    },
    hex_line::HexLine,
    probe_wake::ProbeWake,
    sleep,
    usb::{self, USB, command::Command, line_reader::LineReader},
    wifi::{address::Address, udp_socket::UdpSocket},
};

use core::fmt::Write;
use unor4_hal::wake::{WakeError, WakeTimer};

/// The test's one rule, applied to everything that arrives: stamp it with
/// the clock, then answer.
/// - A 3-byte datagram: `PING <payload> <t>` over USB, `t` the stamp of the
///   look that found it whole. The answer is sent after the stamp, so how
///   long it takes changes nothing measured.
/// - `SYNC <n> …`: `SYNC <n> <t1> <t2>`, `t1` the stamp of the look that
///   found the line ended, `t2` taken just before the answer's first byte.
/// - `STATUS`: the board's address and port, for the PC to aim at.
///
/// Each look takes the stamp first, before reading anything: the board
/// wakes on each byte, so the look that finds a datagram or a line whole
/// is the one its last byte woke, and its stamp is a few µs after that
/// byte. The board does nothing else: no step generator, no ticks, so no
/// other work can delay a look (the rig's tick work, up to 50 µs, is
/// `real-timings.md` gap 8a).
pub(crate) struct Prober {
    clock: Clock,
    udp: UdpSocket,
    address: Address,
    reader: LineReader,
}

impl Prober {
    pub(crate) fn new(clock: Clock, address: Address) -> Self {
        Self {
            clock,
            udp: UdpSocket::new(),
            address,
            reader: LineReader::new(),
        }
    }

    /// Answers for ever; returns only if the wake timer fails.
    pub(crate) fn run(&mut self, wake: &mut WakeTimer) -> Result<(), WakeError> {
        loop {
            let woke = sleep::until(|| {
                look(&self.clock, &mut self.udp, &mut self.reader, wake).transpose()
            })?;
            match woke {
                ProbeWake::Datagram { payload, at } => {
                    let mut line = HexLine::new();
                    line.text(PING_PREFIX)
                        .bytes(&payload)
                        .text(&[FIELD_SEPARATOR])
                        .u32(at)
                        .text(LINE_END);
                    USB.write_all(line.as_bytes());
                }
                ProbeWake::Usb {
                    command: Command::Sync(number),
                    at,
                } => {
                    let mut line = HexLine::new();
                    line.text(SYNC_PREFIX)
                        .u32(number)
                        .text(&[FIELD_SEPARATOR])
                        .u32(at)
                        .text(&[FIELD_SEPARATOR]);
                    let sent = self.clock.now();
                    line.u32(sent).text(LINE_END);
                    USB.write_all(line.as_bytes());
                }
                ProbeWake::Usb {
                    command: Command::Status,
                    ..
                } => self.answer_status(),
            }
        }
    }

    fn answer_status(&self) {
        USB.write_all(REPLY_STATUS);
        USB.write_all(STATUS_RUNNING);
        USB.write_all(self.address.as_bytes());
        // Writing to the USB line never fails (`fmt::Write for &UartIrq`).
        let _ = write!(&mut &USB, " {UDP_PORT}");
        USB.write_all(LINE_END);
    }
}

/// One look, without sleeping: the stamp first, then the datagrams, then
/// the USB line.
fn look(
    clock: &Clock,
    udp: &mut UdpSocket,
    reader: &mut LineReader,
    wake: &mut WakeTimer,
) -> Result<Option<ProbeWake>, WakeError> {
    let at = clock.now();
    while let Some(datagram) = udp.look(wake)? {
        if datagram.size() == PING_SIZE
            && let [a, b, c] = *datagram.payload()
        {
            return Ok(Some(ProbeWake::Datagram {
                payload: [a, b, c],
                at,
            }));
        }
    }
    Ok(usb::look(reader).map(|command| ProbeWake::Usb { command, at }))
}
