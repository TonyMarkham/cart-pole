//! The live calibration's firmware for the rig's Arduino. It joins the WiFi,
//! sleeps, and acts on UDP datagrams: one asks for the level of every pin the
//! rig uses, for the page on the Orin's webserver, and two move the cart
//! slowly to one end of the rail or the other. README.md says what it is for
//! and how to build it.

#![no_std]
#![no_main]

mod answer;
mod buffer;
mod constants;
mod direction;
mod end;
mod error;
mod joined;
mod limit;
mod monitor;
mod moved;
mod pins;
mod rail;
mod seek;
mod settings;
mod sleep;
mod stop_listener;
mod stopwatch;
mod switch;
mod text;
mod trip;
mod wake;
mod wifi;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    monitor::Monitor,
    pins::{D2_SWITCH, D3_SWITCH, Pins, STEPPER, WAKE},
    settings::{AT_BAUD, MONITOR_BAUD, UDP_PORT, WIFI_SSID},
    wifi::Wifi,
};

use core::fmt::{self, Write};
use panic_halt as _;
use unor4_hal::{
    Peripherals,
    gpio::Switch,
    intc,
    step::StepGenerator,
    uart::{Uart, UartIrq},
    wake::WakeTimer,
};

/// The link to the WiFi chip, in interrupt mode. A `static`, because its
/// interrupt handlers reach it (unor4-hal `uart/uart_irq.rs`).
static AT: UartIrq<ra4m1::SCI1> = UartIrq::new();

/// The monitor line (SCI9), from whichever peripherals are at hand: a failed
/// build hands back the ones it was given, and a good one the rest.
macro_rules! monitor {
    ($p:ident) => {
        Monitor(Uart::new(
            $p.sci9,
            $p.sci9_tx,
            $p.sci9_rx,
            MONITOR_BAUD,
            &$p.clocks,
        ))
    };
}

#[cortex_m_rt::entry]
fn main() -> ! {
    // take() succeeds on its first call, and this is the only one.
    let Some(p) = Peripherals::take() else {
        loop {
            cortex_m::asm::wfi();
        }
    };
    let sysclk = p.clocks.sysclk();
    // The drivers that claim from the whole peripherals come first; the
    // fields are taken from what they give back (the HAL's README, *Order*).
    let (stepper, p) = match StepGenerator::new(STEPPER).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => halt(
            monitor!(p),
            format_args!("the step generator could not be built: {error:?}"),
        ),
    };
    let (d2, p) = match Switch::new(D2_SWITCH).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => halt(
            monitor!(p),
            format_args!("the D2 switch could not be built: {error:?}"),
        ),
    };
    let (d3, p) = match Switch::new(D3_SWITCH).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => halt(
            monitor!(p),
            format_args!("the D3 switch could not be built: {error:?}"),
        ),
    };
    let (wake, p) = match WakeTimer::new(WAKE).with_peripherals(p).build() {
        Ok(built) => built,
        Err((error, p)) => halt(
            monitor!(p),
            format_args!("the wake timer could not be built: {error:?}"),
        ),
    };
    let mut delay = p.delay;
    let mut monitor = monitor!(p);

    AT.attach(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);
    // Safety: the handlers are linked, and this is outside any critical
    // section (unor4-hal `uart/uart_irq.rs`). The step generator's, the
    // switches' and the wake timer's wake-ups need them unmasked too: the
    // HAL's vector table acknowledges them.
    unsafe { intc::enable_interrupts() };
    // After the unmask: enabling the driver sleeps on the wake timer.
    let mut pins = match Pins::new(stepper, d2, d3, wake, sysclk) {
        Ok(pins) => pins,
        Err(error) => halt(
            monitor,
            format_args!("the motor driver could not be enabled: {error}"),
        ),
    };

    // The monitor's writes never fail.
    let _ = write!(monitor, "calibration: joining WiFi network {WIFI_SSID}\r\n");
    let mut wifi = match Wifi::join(&AT, &mut delay) {
        Ok((wifi, joined)) => {
            let version = joined
                .bridge_version
                .as_ref()
                .map_or("unknown", |version| version.as_str());
            let _ = write!(monitor, "wifi: bridge {version}; push mode on\r\n");
            let _ = write!(
                monitor,
                "wifi: joined; listening at {} UDP port {UDP_PORT}\r\n",
                joined.address.as_str()
            );
            Some(wifi)
        }
        Err(error) => {
            let _ = write!(monitor, "wifi: {error}; reset the board to try again\r\n");
            None
        }
    };

    loop {
        match wifi.as_mut() {
            Some(wifi) => {
                if let Err(error) = wifi.serve(&mut delay, &mut pins) {
                    // The asker has the answer; this adds where it came from.
                    let _ = write!(monitor, "move: {error}\r\n");
                }
            }
            None => cortex_m::asm::wfi(),
        }
    }
}

/// Says on the monitor line what failed at start-up, and why, then stops the
/// board: without its pins it can do nothing. Each build's one run-time
/// refusal is a full interrupt table, which this firmware's few drivers
/// cannot fill (the HAL's `build` docs). The HAL's errors are printed with
/// `{:?}`, their only format, which costs little: each is a bare name. An
/// `AppError` is printed with `{}`, the message the main loop already prints:
/// its `{:?}` would link about 9.5 KB of formatting code for this one line
/// (measured with `cargo nm`, 2026-11-04).
fn halt(mut monitor: Monitor, failure: fmt::Arguments) -> ! {
    // The monitor's writes never fail.
    let _ = write!(monitor, "calibration: {failure}\r\n");
    loop {
        cortex_m::asm::wfi();
    }
}
