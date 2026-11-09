//! Test firmware for the bench UNO R4 WiFi (`__workbench__/README.md`, *The
//! apply test*). It runs the rig controller's MOVING loop on the controller's
//! step generator, following 3-byte commands that arrive over WiFi, and flips
//! D3 the moment each command is whole. The R3 on the bench times that edge
//! and every STEP pulse on its Timer1, so the time from a command's arrival
//! to its new rate on STEP, gap 8 of `real-timings.md`, is measured on one
//! clock, the R3's. Nothing moves: the bench's STEP and DIR go only to the R3.

#![no_std]
#![no_main]

mod config;
mod constants;
mod pins;
mod run;
mod sleep;
mod stepper;
mod usb;
mod wake_timer;
mod wifi;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    config::{usb::USB_BAUD, wifi::AT_BAUD},
    constants::{LINE_END, LINE_READY, REPLY_STATUS, STATUS_JOINING},
    pins::mark::Mark,
    run::runner::Runner,
    stepper::STEPPER,
    usb::{USB, command::Command, line_reader::LineReader},
    wake_timer::WAKE_TIMER,
    wifi::AT,
};

use core::fmt::{self, Write};
use panic_halt as _;
use unor4_hal::{Peripherals, intc, step::StepGenerator, wake::WakeTimer};

/// Opens the USB line from the peripherals a failed build handed back, then
/// answers every `STATUS` with why the board stopped.
macro_rules! fail {
    ($p:ident, $($why:tt)*) => {{
        USB.attach($p.sci9, $p.sci9_tx, $p.sci9_rx, USB_BAUD, &$p.clocks);
        // Safety: no critical section is open here
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

    let (mut wake_timer, p) = match WakeTimer::new(WAKE_TIMER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "wake timer: {:?}", error),
    };

    // The fields taken directly come last: this leaves `p` partly moved.
    let mark = Mark::new(p.pins.d3.into_output());
    AT.attach(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);
    USB.attach(p.sci9, p.sci9_tx, p.sci9_rx, USB_BAUD, &p.clocks);
    // Safety: no critical section is open here (unor4-hal
    // `intc/enable_interrupts.rs`). The bootloader leaves interrupts masked;
    // the USB line's handlers and the timers' wake-ups need them on.
    unsafe { intc::enable_interrupts() };

    USB.write_all(LINE_READY);
    // Asked during the join, the board can't answer: the join's waits look
    // only at the WiFi chip. Said once here, so a PC already listening knows.
    USB.write_all(REPLY_STATUS);
    USB.write_all(STATUS_JOINING);
    USB.write_all(LINE_END);

    let address = match wifi::join::join(&mut wake_timer) {
        Ok((_socket, address)) => address,
        Err(error) => stopped(format_args!("wifi: {error}")),
    };

    let mut runner = Runner::new(stepper, mark, address);
    match runner.run(&mut wake_timer) {
        Ok(()) => stopped(format_args!("run: ended")),
        Err(error) => stopped(format_args!("run: {error}")),
    }
}

/// Stops the board for good: it sleeps, and nothing makes it do more.
fn halt() -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}

/// Stops the board for good, answering each `STATUS` with
/// `OK STATUS stopped <why>`.
fn stopped(why: fmt::Arguments) -> ! {
    let mut reader = LineReader::new();
    loop {
        // `STATUS` is the only line the board knows.
        let Command::Status = sleep::until(|| usb::look(&mut reader));
        // Writing to the USB line never fails (`fmt::Write for &UartIrq`).
        let _ = writeln!(&mut &USB, "OK STATUS stopped {why}");
    }
}
