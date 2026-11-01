//! The live calibration's firmware for the rig's Arduino. For now it does
//! one thing: it joins the WiFi, sleeps, and when a UDP datagram asks for
//! them, answers with the level of every pin the rig uses, for the page on
//! the Orin's webserver. It sends no pulses and acts on no request, so it
//! cannot move the cart. README.md says what it is for and how to build it.

#![no_std]
#![no_main]

mod buffer;
mod pins;
mod settings;
mod wifi;

use core::fmt::{self, Write};

use panic_halt as _;
use unor4_hal::{
    Peripherals, intc,
    uart::{Uart, UartIrq},
};

use crate::{
    pins::Pins,
    settings::{MONITOR_BAUD, UDP_PORT, WIFI_SSID},
    wifi::Wifi,
};

/// The AT link's baud: the bridge firmware opens its AT server at
/// 115,200 8N1 (uno-r4-wifi-usb-bridge, `UNOR4USBBridge.ino`, SERIAL_AT).
const AT_BAUD: u32 = 115_200;

/// The link to the WiFi chip, in interrupt mode. A `static`, because its
/// interrupt handlers reach it (unor4-hal `uart/uart_irq.rs`).
static AT: UartIrq<ra4m1::SCI1> = UartIrq::new();

/// Lines of text on the monitor line (SCI9), which the board's ESP32-S3
/// forwards to its USB serial port. `Uart::write_bytes` cannot fail.
struct Monitor(Uart<ra4m1::SCI9>);

impl Write for Monitor {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write_bytes(text.as_bytes());
        Ok(())
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    // take() succeeds on its first call, and this is the only one.
    let Some(p) = Peripherals::take() else {
        loop {
            cortex_m::asm::wfi();
        }
    };
    let mut delay = p.delay;
    let mut monitor = Monitor(Uart::sci9(p.sci9, p.sci9_tx, p.sci9_rx, MONITOR_BAUD, &p.clocks));
    let pins = Pins::new(p.pins.d2, p.pins.d3, p.pins.d4, p.pins.d5, p.pins.d6);

    AT.sci1(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);
    // Safety: the handlers are linked, and this is outside any critical
    // section (unor4-hal `uart/uart_irq.rs`).
    unsafe { intc::enable_interrupts() };

    // The monitor's writes never fail.
    let _ = write!(monitor, "calibration: joining WiFi network {WIFI_SSID}\r\n");
    let mut wifi = match Wifi::join(&AT, &mut delay) {
        Ok((wifi, joined)) => {
            let version = joined.bridge_version.as_ref().map_or("unknown", |version| version.as_str());
            let _ = write!(monitor, "wifi: bridge {version}; push mode on\r\n");
            let _ = write!(monitor, "wifi: joined; listening at {} UDP port {UDP_PORT}\r\n", joined.address.as_str());
            Some(wifi)
        }
        Err(error) => {
            let _ = write!(monitor, "wifi: {}; reset the board to try again\r\n", error.describe());
            None
        }
    };

    loop {
        match wifi.as_mut() {
            Some(wifi) => wifi.serve(&mut delay, &pins),
            None => cortex_m::asm::wfi(),
        }
    }
}
