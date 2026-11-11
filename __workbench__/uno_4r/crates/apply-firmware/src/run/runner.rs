use crate::{
    config::wifi::UDP_PORT,
    constants::{BENCH_STOP_POINT, LINE_END, REPLY_STATUS, STATUS_RUNNING},
    pins::mark::Mark,
    run::{command, ramp::Ramp, run_error::RunError, run_wake::RunWake, stop_points::StopPoints},
    sleep,
    usb::{self, USB, command::Command, line_reader::LineReader},
    wifi::{address::Address, udp_socket::UdpSocket},
};

use core::fmt::Write;
use unor4_hal::{
    step::{StepError, StepGenerator},
    wake::WakeTimer,
};

/// The controller's MOVING loop (its `cart/mod.rs`, `until_at_rest`), run for
/// ever, with D3 flipped the moment each command is whole, for the R3. Each
/// wake does what the controller's does: on a tick, take the count, check it
/// against the stop points, step the ramp, cap it by the soft limit, and
/// queue it; on a datagram, re-aim the ramp. So the time from the mark to
/// the new rate on STEP is the controller's gap 8 (`__documentation__/real-timings.md`). The
/// differences, each a few clock cycles at most:
/// - no limit switches to read (the controller reads two switches first);
/// - the stop points are `BENCH_STOP_POINT` away, never reached;
/// - the run never ends at rest, so every tick is a MOVING tick;
/// - a `STATUS` line over USB is answered, which only the PC sends, and
///   only before the measurement starts.
pub(crate) struct Runner {
    stepper: StepGenerator,
    mark: Mark,
    udp: UdpSocket,
    address: Address,
    reader: LineReader,
}

impl Runner {
    pub(crate) fn new(stepper: StepGenerator, mark: Mark, address: Address) -> Self {
        Self {
            stepper,
            mark,
            udp: UdpSocket::new(),
            address,
            reader: LineReader::new(),
        }
    }

    /// Runs the commands until something fails; never returns otherwise.
    pub(crate) fn run(&mut self, wake: &mut WakeTimer) -> Result<(), RunError> {
        let stops = StopPoints::new(-BENCH_STOP_POINT, BENCH_STOP_POINT);
        let mut ramp = Ramp::new();

        self.stepper.enable();
        self.stepper.set_tick_wake(true);
        let base = sleep::until(|| self.stepper.poll_tick().map(|_| self.stepper.position()));

        loop {
            let woke = sleep::until(|| {
                look(
                    &mut self.stepper,
                    &mut self.udp,
                    &mut self.mark,
                    &mut self.reader,
                    wake,
                )
                .transpose()
            })?;
            match woke {
                RunWake::Command(rate) => ramp.command(rate),
                RunWake::Usb(Command::Status) => self.answer_status(),
                RunWake::Tick => {
                    let position = self.stepper.position() - base;
                    // The controller ends the move here; the bench's stop
                    // points are never passed, so the check only costs what
                    // it costs the controller.
                    if stops.passed(position) {
                        ramp.command(0);
                    }
                    let next = ramp.next();
                    let sent = stops.cap(next, ramp.rate(), position);
                    match self.stepper.queue(sent) {
                        Ok(_) => ramp.sent(sent),
                        // Not this tick: the rate running now holds for one
                        // more, and the ramp tries again at the next.
                        Err(StepError::NotReady | StepError::TooLate) => {}
                        Err(error) => return Err(error.into()),
                    }
                }
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

/// One look, without sleeping, in the controller's order: the tick first,
/// since the rate for the next tick must be queued at least 40 µs before it
/// (unor4-hal `step/step_generator.rs`, `queue`), then the datagrams, then
/// the USB line. The mark is flipped as soon as a command is found whole,
/// before anything acts on it: that is where gap 8 starts.
fn look(
    stepper: &mut StepGenerator,
    udp: &mut UdpSocket,
    mark: &mut Mark,
    reader: &mut LineReader,
    wake: &mut WakeTimer,
) -> Result<Option<RunWake>, RunError> {
    if stepper.poll_tick().is_some() {
        return Ok(Some(RunWake::Tick));
    }
    while let Some(datagram) = udp.look(wake)? {
        if let Some(rate) = command::rate(&datagram) {
            mark.flip();
            return Ok(Some(RunWake::Command(rate)));
        }
    }
    Ok(usb::look(reader).map(RunWake::Usb))
}
