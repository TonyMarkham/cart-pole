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
mod stopwatch;
mod switch;
mod text;
mod trip;
mod wifi;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    monitor::Monitor,
    pins::Pins,
    settings::{AT_BAUD, MONITOR_BAUD, UDP_PORT, WIFI_SSID},
    wifi::Wifi,
};

use core::fmt::Write;
use panic_halt as _;
use unor4_hal::{
    Peripherals, intc,
    uart::{Uart, UartIrq},
};

/// The link to the WiFi chip, in interrupt mode. A `static`, because its
/// interrupt handlers reach it (unor4-hal `uart/uart_irq.rs`).
static AT: UartIrq<ra4m1::SCI1> = UartIrq::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    // take() succeeds on its first call, and this is the only one.
    let Some(p) = Peripherals::take() else {
        loop {
            cortex_m::asm::wfi();
        }
    };
    let mut delay = p.delay;
    let mut monitor = Monitor(Uart::sci9(
        p.sci9,
        p.sci9_tx,
        p.sci9_rx,
        MONITOR_BAUD,
        &p.clocks,
    ));
    let mut pins = Pins::new(
        p.pins.d2,
        p.pins.d3,
        p.pins.d4,
        p.pins.d5,
        p.pins.d6,
        p.clocks.sysclk(),
    );

    AT.sci1(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);
    // Safety: the handlers are linked, and this is outside any critical
    // section (unor4-hal `uart/uart_irq.rs`).
    unsafe { intc::enable_interrupts() };

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
