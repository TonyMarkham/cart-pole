//! Test firmware for the bench UNO R4 WiFi (`__workbench__/README.md`, *The
//! WiFi test*). It times nothing itself: it stamps, on a free-running
//! 48 MHz timer, each 3-byte datagram the moment it has it whole, and each
//! clock-sync line from the PC, and reports the stamps over USB. The PC
//! synchronises this clock to its own from the syncs, and so times the WiFi
//! hop, from its send to the board having the datagram, on one clock, the
//! PC's (`real-timings.md`, gap 7).

#![no_std]
#![no_main]

mod clock;
mod config;
mod constants;
mod hex_line;
mod probe_wake;
mod prober;
mod sleep;
mod usb;
mod wake_timer;
mod wifi;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    clock::Clock,
    config::{usb::USB_BAUD, wifi::AT_BAUD},
    constants::{LINE_END, LINE_READY, REPLY_STATUS, STATUS_JOINING},
    prober::Prober,
    usb::{USB, command::Command, line_reader::LineReader},
    wake_timer::WAKE_TIMER,
    wifi::AT,
};

use core::fmt::{self, Write};
use panic_halt as _;
use unor4_hal::{Peripherals, intc, wake::WakeTimer};

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

    let (mut wake_timer, p) = match WakeTimer::new(WAKE_TIMER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => fail!(p, "wake timer: {:?}", error),
    };

    // The fields taken directly come last: this leaves `p` partly moved.
    let clock = Clock::new(p.gpt320);
    AT.attach(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);
    USB.attach(p.sci9, p.sci9_tx, p.sci9_rx, USB_BAUD, &p.clocks);
    // Safety: no critical section is open here (unor4-hal
    // `intc/enable_interrupts.rs`). The bootloader leaves interrupts masked;
    // the USB line's handlers and the wake timer need them on.
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

    let mut prober = Prober::new(clock, address);
    match prober.run(&mut wake_timer) {
        Ok(()) => stopped(format_args!("probe: ended")),
        Err(error) => stopped(format_args!("probe: wake timer: {error:?}")),
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
        if let Command::Status = sleep::until(|| usb::look(&mut reader)) {
            // Writing to the USB line never fails (`fmt::Write for &UartIrq`).
            let _ = writeln!(&mut &USB, "OK STATUS stopped {why}");
        }
    }
}
