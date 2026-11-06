//! The rig's Arduino firmware that runs the cart under the Orin's commands.
//! It is built one slice at a time. This first slice is the board's state
//! machine (`State`, `Event`); the board starts in its first state and
//! sleeps, since nothing drives the machine yet.

#![no_std]
#![no_main]

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
    config::{usb::USB_BAUD, wifi::AT_BAUD},
    constants::{LINE_END, LINE_READY, REPLY_STATUS},
    pins::{idler_limit_switch::IDLER_LIMIT_SWITCH, motor_limit_switch::MOTOR_LIMIT_SWITCH},
    state::State,
    stepper::STEPPER,
    usb::{USB, command::Command as UsbCommand, line_reader::LineReader},
    wake_timer::WAKE_TIMER,
    wifi::AT,
};

use crate::incoming::Incoming;
use crate::wifi::udp_socket::UdpSocket;
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

    let (_stepper, p) = match StepGenerator::new(STEPPER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "step generator: {:?}", error),
    };

    let (_idler, p) = match Switch::new(IDLER_LIMIT_SWITCH).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "idler switch: {:?}", error),
    };

    let (_motor, p) = match Switch::new(MOTOR_LIMIT_SWITCH).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "motor switch: {:?}", error),
    };

    let (mut wake_timer, p) = match WakeTimer::new(WAKE_TIMER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "wake timer: {:?}", error),
    };

    AT.attach(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);

    let mut udp = match wifi::join::join(&mut wake_timer) {
        Ok(socket) => UdpSocket::new(socket),
        Err(error) => stopped(format_args!("wifi: {error}")),
    };

    // The fields taken directly come last: this leaves `p` partly moved.
    USB.attach(p.sci9, p.sci9_tx, p.sci9_rx, USB_BAUD, &p.clocks);
    // Safety: no critical section is open here (unor4-hal
    // `intc/enable_interrupts.rs`). The bootloader leaves interrupts masked;
    // the USB line's handlers, and later the switches' and the step
    // generator's wake-ups, need them on.
    unsafe { intc::enable_interrupts() };

    USB.write_all(LINE_READY);

    let _socket = match wifi::join::join(&mut wake_timer) {
        Ok(socket) => socket,
        Err(error) => stopped(format_args!("wifi: {error}")),
    };

    let state = State::START;
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
            Ok(Incoming::Wifi(datagram)) => {
                if let Err(error) = wifi::serve(&mut udp, &mut wake_timer, state, &datagram) {
                    stopped(format_args!("wifi: wake timer: {error:?}"));
                }
            }
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
