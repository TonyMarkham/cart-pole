//! The rig's Arduino firmware that runs the cart under the Orin's commands.
//! It is built one slice at a time. So far: the board's state machine
//! (`State`, `Event`), `STATUS` over USB, `state` and commands over WiFi, and
//! the first calibration move, the seek of the idler's end.

#![no_std]
#![no_main]

mod cart;
mod config;
mod constants;
mod incoming;
mod pins;
mod sleep;
mod state;
mod stepper;
mod usb;
mod wake_timer;
mod wifi;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    cart::{Cart, move_timer::MOVE_TIMER},
    config::{usb::USB_BAUD, wifi::AT_BAUD},
    constants::{LINE_END, LINE_READY, REPLY_STATUS},
    incoming::Incoming,
    pins::{idler_limit_switch::IDLER_LIMIT_SWITCH, motor_limit_switch::MOTOR_LIMIT_SWITCH},
    state::{State, event::Event},
    stepper::STEPPER,
    usb::{USB, command::Command as UsbCommand, line_reader::LineReader},
    wake_timer::WAKE_TIMER,
    wifi::{AT, answer::Answer, message::Message as WifiMessage, udp_socket::UdpSocket},
};

use core::fmt::{self, Write};
use panic_halt as _;
use unor4_hal::{Peripherals, gpio::Switch, intc, step::StepGenerator, wake::WakeTimer};

/// Opens the USB line from the peripherals a failed build handed back, then
/// answers every `STATUS` with why the board stopped.
macro_rules! fail {
    ($p:ident, $($why:tt)*) => {{
        USB.attach($p.sci9, $p.sci9_tx, $p.sci9_rx, USB_BAUD, &$p.clocks);
        // Safety: no critical section is open here:
        // (unor4-hal `intc/enable_interrupts.rs`).
        unsafe { intc::enable_interrupts() };
        stopped(format_args!($($why)*))
    }};
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let Some(p) = Peripherals::take() else {
        halt();
    };

    let (stepper, p) = match StepGenerator::new(STEPPER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "step generator: {:?}", error),
    };

    let (idler, p) = match Switch::new(IDLER_LIMIT_SWITCH).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "idler switch: {:?}", error),
    };

    let (motor, p) = match Switch::new(MOTOR_LIMIT_SWITCH).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "motor switch: {:?}", error),
    };

    let (mut wake_timer, p) = match WakeTimer::new(WAKE_TIMER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "wake timer: {:?}", error),
    };

    let (move_timer, p) = match WakeTimer::new(MOVE_TIMER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "move timer: {:?}", error),
    };

    let mut cart = Cart::new(stepper, idler, motor, move_timer);

    AT.attach(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);

    // The fields taken directly come last: this leaves `p` partly moved.
    USB.attach(p.sci9, p.sci9_tx, p.sci9_rx, USB_BAUD, &p.clocks);
    // Safety: no critical section is open here (unor4-hal
    // `intc/enable_interrupts.rs`). The bootloader leaves interrupts masked;
    // the USB line's handlers, and the switches' and timers' wake-ups, need
    // them on.
    unsafe { intc::enable_interrupts() };

    USB.write_all(LINE_READY);

    let mut udp = match wifi::join::join(&mut wake_timer) {
        Ok(socket) => UdpSocket::new(socket),
        Err(error) => stopped(format_args!("wifi: {error}")),
    };

    let mut state = State::START;
    let mut reader = LineReader::new();
    loop {
        let incoming = sleep::until(|| match usb::look(&mut reader) {
            Some(command) => Some(Ok(Incoming::Usb(command))),
            None => udp
                .look(&mut wake_timer)
                .map(|datagram| datagram.map(Incoming::Wifi))
                .transpose(),
        });

        match incoming {
            Ok(Incoming::Usb(UsbCommand::Status)) => {
                USB.write_all(REPLY_STATUS);
                USB.write_all(state.name().as_bytes());
                USB.write_all(LINE_END);
            }
            Ok(Incoming::Wifi(datagram)) => match WifiMessage::parse(&datagram) {
                // A seek is a calibration move, so it runs from every state
                // that leads to CALIBRATING, and CALIBRATING itself, between
                // moves (real-open-issues.md, issue 5, step 1).
                Some(WifiMessage::Seek(toward))
                    if matches!(
                        state,
                        State::Uncalibrated | State::Calibrating | State::Calibrated
                    ) =>
                {
                    // Who asked, read before the move: the answer goes there,
                    // not to the `Stop` that may end it. Unknown, the cart
                    // stays put, since no answer could reach the asker.
                    let asker = match udp.asker(&mut wake_timer) {
                        Ok(Some(asker)) => asker,
                        Ok(None) => continue,
                        Err(error) => stopped(format_args!("wifi: wake timer: {error:?}")),
                    };
                    state = state.next(Event::StartCalibrating);
                    let (end, pulses) = match cart.seek(toward, &mut udp, &mut wake_timer) {
                        Ok(seek) => seek,
                        Err(error) => stopped(format_args!("seek: {error}")),
                    };
                    let answer = Answer::seek(toward, end, pulses);
                    if let Err(error) = wifi::send_to(&mut udp, &mut wake_timer, &asker, &answer) {
                        stopped(format_args!("wifi: wake timer: {error:?}"));
                    }
                }
                message => {
                    if let Err(error) = wifi::serve(&mut udp, &mut wake_timer, state, message) {
                        stopped(format_args!("wifi: wake timer: {error:?}"));
                    }
                }
            },
            Err(error) => stopped(format_args!("wifi: wake timer: {error:?}")),
        }
    }
}

/// Stops the board for good: it sleeps, and nothing makes it do more. A
/// failed build can't say why yet; that needs the USB serial line.
fn halt() -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}

/// Stops the board for good, answering each `STATUS` with
/// `OK STATUS stopped <why>`. Asked, not announced: a line sent at the start
/// falls inside the manager's baud change and is thrown away.
fn stopped(why: fmt::Arguments) -> ! {
    let mut reader = LineReader::new();
    loop {
        match usb::next_command(&mut reader) {
            // Writing to the USB line never fails (`fmt::Write for &UartIrq`).
            UsbCommand::Status => {
                let _ = writeln!(&mut &USB, "OK STATUS stopped {why}");
            }
        }
    }
}
