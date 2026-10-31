//! Homes the cart between the rig's two limit switches and centres it,
//! then waits for the buttons on its web page. The stepper reports
//! nothing about where it is, so the only way to find the centre is to
//! measure the rail in pulses: seek slowly in Minus until either switch
//! trips (the first end, pulse 0), seek slowly in Plus until a switch
//! trips again (the second end) while counting every pulse, then drive
//! half that count back, fast, on a ramp so the motor does not slip.
//! Which switch is at which end, and which way DIR moves the cart, do
//! not matter: either switch is an end. The report names the pin at each
//! end, and both ends on one pin halts as a wiring fault.
//!
//! The centre is halfway between the two switches' trip points, both
//! approached moving outward at the same slow rate, so any lag in
//! sensing a trip is the same at both ends and cancels.
//!
//! Before moving, the board joins the WiFi network in `wifi.toml` and
//! serves a page at `http://<board IP>:<port>/` with the controls
//! (Calibrate, Go to centre, jog, enable, disable), a big STOP, and every
//! fact the run found. A button's answer waits until its action ends,
//! so the browser sends nothing during a move and then shows the result.
//! STOP is checked every `stop_check_ms` during the slow seeks and jogs; the fast ramp to the centre cannot pause, so a
//! STOP pressed during it takes effect when it ends. A failed join is
//! reported and the program goes on without the page.
//!
//! - Wiring: control-design.md (D2 and D3 the switches, D4 STEP, D5 DIR,
//!   D6 ENABLE, D8 FAULT).
//! - Settings: calibrate.toml and wifi.toml, compiled in by build.rs.
//!   With `run_at_start`, every power-up or reset calibrates by itself;
//!   without it, the board waits for the Calibrate button.
//! - Report: progress, the board's address and the measured span on the
//!   board's USB serial port and on the page. D13's LED lights steady
//!   while the cart is homed, and flashes fast after a halt that switched
//!   the driver off.
//!
//! Build and upload: README.md.

#![no_std]
#![no_main]

mod action;
mod axis;
mod buffer;
mod direction;
mod experiment;
mod halt;
mod kind;
mod monitor;
mod outcome;
mod page;
mod pins;
mod profile;
mod report;
mod settings;
mod stopwatch;
mod switch;
mod trip;
mod wifi;

use core::fmt::Write;

use embedded_hal::delay::DelayNs;
use panic_halt as _;
use unor4_hal::{Peripherals, delay::Delay, uart::Uart};

use crate::{
    action::Action,
    axis::Axis,
    direction::Direction,
    halt::Halt,
    monitor::Monitor,
    outcome::Outcome,
    report::{Report, millimetres},
    settings::{HOME_SPEED, HTTP_PORT, JOG_PULSES, MONITOR_BAUD, RUN_AT_START, SEEK_SPEED, SETTLE_MS, START_MS, WIFI_SSID},
    switch::Switch,
    wifi::Wifi,
};

/// The AT link's baud: the bridge firmware opens its AT server at
/// 115,200 8N1 (uno-r4-wifi-usb-bridge, `UNOR4USBBridge.ino`, SERIAL_AT).
const AT_BAUD: u32 = 115_200;

/// The idle loop's period, ms: how often it checks for a request, and
/// the LED's half-period after a halt (fast enough to tell apart from the
/// steady "homed" light).
const IDLE_TICK_MS: u32 = 100;

#[cortex_m_rt::entry]
fn main() -> ! {
    // take() succeeds on its first call, and this is the only one.
    let Some(p) = Peripherals::take() else {
        loop {
            cortex_m::asm::wfi();
        }
    };
    let mut delay = p.delay;
    let mut monitor = Monitor::new(Uart::sci9(p.sci9, p.sci9_tx, p.sci9_rx, MONITOR_BAUD, &p.clocks));
    let mut led = p.pins.d13.into_output();
    let mut axis = Axis::new(
        p.pins.d4,
        p.pins.d5,
        p.pins.d6,
        p.pins.d2,
        p.pins.d3,
        p.pins.d8,
        p.clocks.sysclk(),
    );

    monitor.line(format_args!("calibrate: joining WiFi network {WIFI_SSID}"));
    let at_link = Uart::sci1(p.sci1, p.sci1_tx, p.sci1_rx, AT_BAUD, &p.clocks);
    let mut wifi = match Wifi::join(at_link, &mut delay) {
        Ok((wifi, address)) => {
            monitor.line(format_args!("wifi: joined; the page is at http://{}:{HTTP_PORT}/", address.as_str()));
            Some(wifi)
        }
        Err(error) => {
            monitor.line(format_args!("wifi: {}; going on without WiFi", error.describe()));
            None
        }
    };

    let mut report = Report::default();
    let mut next = None;
    if RUN_AT_START {
        monitor.line(format_args!(
            "calibrate: homing in {START_MS} ms (seek {SEEK_SPEED} pulses/s, centre move up to {HOME_SPEED} pulses/s)"
        ));
        delay.delay_ms(START_MS);
        next = Some(Action::Calibrate);
    } else {
        monitor.line(format_args!("calibrate: waiting for the page's Calibrate button"));
    }

    loop {
        if let Some(action) = next.take() {
            run(action, &mut axis, &mut delay, &mut monitor, &mut report, &mut wifi);
            if let Some(wifi) = wifi.as_mut() {
                wifi.finish(&mut delay);
            }
        }
        if let Some(wifi) = wifi.as_mut() {
            next = wifi.serve(&mut delay, &report, axis.pins(), monitor.log());
        }
        match report.last {
            Some((_, Outcome::Halted(halt))) if halt.frees_cart() => led.toggle(),
            _ if report.homed => led.set_high(),
            _ => led.set_low(),
        }
        delay.delay_ms(IDLE_TICK_MS);
    }
}

/// Runs one button's action and records how it ended. A halt that
/// frees the cart switches the driver off and forgets the homing.
fn run(
    action: Action,
    axis: &mut Axis,
    delay: &mut Delay,
    monitor: &mut Monitor,
    report: &mut Report,
    wifi: &mut Option<Wifi>,
) {
    if action == Action::Stop {
        monitor.line(format_args!("STOP: nothing is moving"));
        return;
    }
    monitor.line(format_args!("{}", action.label()));
    let mut stop = |delay: &mut Delay| wifi.as_mut().is_some_and(|wifi| wifi.stop_requested(delay, action, None));
    let outcome = match action {
        Action::Calibrate => calibrate(axis, delay, monitor, report, &mut stop),
        Action::Centre => centre(axis, delay, monitor, report),
        Action::JogMinus => jog(axis, delay, monitor, Direction::Minus, &mut stop),
        Action::JogPlus => jog(axis, delay, monitor, Direction::Plus, &mut stop),
        Action::Experiment => experiment::run(axis, delay, monitor, report, wifi),
        Action::Enable => {
            axis.enable(delay);
            Ok(Outcome::Enabled)
        }
        Action::Disable => {
            axis.disable();
            report.homed = false;
            Ok(Outcome::Disabled)
        }
        Action::Stop => return,
    };
    let outcome = outcome.unwrap_or_else(|halt| {
        if halt.frees_cart() {
            axis.disable();
            report.homed = false;
        }
        // The monitor's writes never fail (monitor.rs).
        let _ = write!(monitor, "{}: ", if halt.frees_cart() { "FAULT" } else { "stopped" });
        let _ = halt.describe(monitor);
        monitor.line(format_args!(""));
        if halt.frees_cart() {
            monitor.line(format_args!("halted: driver off, the cart is free; press Calibrate to run again"));
        }
        Outcome::Halted(halt)
    });
    report.position = axis.position();
    report.last = Some((action, outcome));
}

/// First end, second end, then half the span back. Each finding goes
/// into `report` as it is made, so a halt leaves what came before it.
fn calibrate(
    axis: &mut Axis,
    delay: &mut Delay,
    monitor: &mut Monitor,
    report: &mut Report,
    stop: &mut dyn FnMut(&mut Delay) -> bool,
) -> Result<Outcome, Halt> {
    report.forget_homing();
    if let Some(switch) = Switch::ALL.into_iter().find(|&switch| axis.pressed_steady(switch)) {
        return Err(Halt::PressedAtStart { switch });
    }
    axis.enable(delay);

    monitor.line(format_args!("seeking the first end (minus)"));
    let (sent, first) = axis.seek(delay, Direction::Minus, stop)?;
    axis.zero();
    report.first = Some((first, sent));
    monitor.line(format_args!("{} tripped after {sent} pulses: the first end, pulse 0", first.name()));
    delay.delay_ms(SETTLE_MS);

    monitor.line(format_args!("seeking the second end (plus)"));
    let (_, second) = axis.seek(delay, Direction::Plus, stop)?;
    if second == first {
        return Err(Halt::SameSwitch { switch: second });
    }
    // The seek ran from pulse 0, so the count is the span.
    let span = axis.position().unsigned_abs();
    report.second = Some(second);
    report.span = Some(span);
    let (mm, um) = millimetres(span);
    monitor.line(format_args!(
        "{} tripped at pulse {span}: the second end; the span is {span} pulses, {mm}.{um:03} mm",
        second.name()
    ));
    delay.delay_ms(SETTLE_MS);

    let back = span / 2;
    monitor.line(format_args!("moving {back} pulses minus to the centre"));
    axis.travel(delay, Direction::Minus, back)?;
    report.homed = true;
    monitor.line(format_args!("centred at pulse {} of {span}; holding", axis.position()));
    Ok(Outcome::Centred { position: axis.position() })
}

/// From wherever the cart is to half the span, fast. Needs a finished
/// homing: without one the count says nothing about where the cart is.
fn centre(axis: &mut Axis, delay: &mut Delay, monitor: &mut Monitor, report: &Report) -> Result<Outcome, Halt> {
    let Some(span) = report.span.filter(|_| report.homed) else {
        return Err(Halt::NotHomed);
    };
    axis.enable(delay);
    let to_go = (span / 2) as i32 - axis.position();
    let direction = if to_go < 0 { Direction::Minus } else { Direction::Plus };
    monitor.line(format_args!("moving {} pulses {} to the centre", to_go.unsigned_abs(), direction.name()));
    axis.travel(delay, direction, to_go.unsigned_abs())?;
    monitor.line(format_args!("centred at pulse {} of {span}; holding", axis.position()));
    Ok(Outcome::Centred { position: axis.position() })
}

/// `jog_pulses` in `direction` at the seek rate, stopping early at a
/// switch. The count stays good, so a homed cart stays homed.
fn jog(
    axis: &mut Axis,
    delay: &mut Delay,
    monitor: &mut Monitor,
    direction: Direction,
    stop: &mut dyn FnMut(&mut Delay) -> bool,
) -> Result<Outcome, Halt> {
    axis.enable(delay);
    let at_switch = axis.jog(delay, direction, stop)?;
    match at_switch {
        Some(switch) => monitor.line(format_args!("jog stopped at pulse {}: {} tripped", axis.position(), switch.name())),
        None => monitor.line(format_args!("jogged {JOG_PULSES} pulses {} to pulse {}", direction.name(), axis.position())),
    }
    Ok(Outcome::Jogged { position: axis.position(), at_switch })
}
