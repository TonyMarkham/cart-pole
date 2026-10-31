use core::fmt::{self, Write};

use crate::{
    action::Action,
    experiment::{ExperimentReport, JumpLimit},
    outcome::Outcome,
    settings::{
        ACCEL, BELT_UM_PER_REV, DIR_SETUP_NS, END_MARGIN_PULSES, HOLD_MS, HOME_SPEED, HTTP_PORT, J_GROWTH_PERCENT,
        J_START, JOG_PULSES, NOISE_AWAY_PULSES, PULSE_LOW_NS, PULSES_PER_REV, STEADY_MS,
        RELEASE_PULSES, RUN_AT_START, TRIP_US, SEEK_SPEED, SEEK_TIMEOUT_MS, SETTLE_MS, START_MS, STOP_CHECK_MS, WIFI_SSID,
    },
    switch::Switch,
};

/// The settings this build was compiled with (calibrate.toml, wifi.toml
/// and [drive]), so the page shows which build is running.
pub fn write_settings(out: &mut impl Write) -> fmt::Result {
    writeln!(out, "run_at_start   {RUN_AT_START}")?;
    writeln!(out, "start_ms       {START_MS}")?;
    writeln!(out, "seek_speed     {SEEK_SPEED} pulses/s")?;
    writeln!(out, "home_speed     {HOME_SPEED} pulses/s")?;
    writeln!(out, "accel          {ACCEL} pulses/s²")?;
    writeln!(out, "seek_timeout   {SEEK_TIMEOUT_MS} ms")?;
    writeln!(out, "jog_pulses     {JOG_PULSES} pulses")?;
    writeln!(out, "release_pulses {RELEASE_PULSES} pulses")?;
    writeln!(out, "trip_us        {TRIP_US}")?;
    writeln!(out, "stop_check_ms  {STOP_CHECK_MS}")?;
    writeln!(out, "settle_ms      {SETTLE_MS}")?;
    writeln!(out, "pulse_low_ns   {PULSE_LOW_NS}, dir_setup_ns {DIR_SETUP_NS}")?;
    writeln!(
        out,
        "experiment     j_start {J_START}, +{J_GROWTH_PERCENT}% a step, steady {STEADY_MS} ms, hold {HOLD_MS} ms"
    )?;
    writeln!(out, "               noise_away {NOISE_AWAY_PULSES} pulses, end_margin {END_MARGIN_PULSES} pulses")?;
    writeln!(out, "drive          {PULSES_PER_REV} pulses per turn, {BELT_UM_PER_REV} µm of belt per turn")?;
    writeln!(out, "wifi           {WIFI_SSID}, port {HTTP_PORT}")
}

/// What the program has found and done: the same facts the serial port
/// prints, kept so the WiFi page can show them.
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    /// The first end's switch, and the pulses the first seek sent to
    /// reach it.
    pub first: Option<(Switch, u32)>,
    /// The second end's switch.
    pub second: Option<Switch>,
    /// Pulses from the first end to the second.
    pub span: Option<u32>,
    /// Whether the count says where the cart is: true from a finished
    /// homing until the driver is switched off.
    pub homed: bool,
    /// The count when the last action ended.
    pub position: i32,
    /// The last action and how it ended.
    pub last: Option<(Action, Outcome)>,
    /// The jump-limit experiment's findings.
    pub experiment: ExperimentReport,
}

impl Report {
    /// Forgets the last homing's findings, as a new one starts.
    pub fn forget_homing(&mut self) {
        self.first = None;
        self.second = None;
        self.span = None;
        self.homed = false;
    }

    pub fn write(&self, out: &mut impl Write) -> fmt::Result {
        match self.first {
            Some((switch, sent)) => {
                writeln!(out, "first end:  {} tripped after {sent} pulses (that point is pulse 0)", switch.name())?
            }
            None => writeln!(out, "first end:  not found")?,
        }
        match (self.second, self.span) {
            (Some(switch), Some(span)) => {
                writeln!(out, "second end: {} tripped at pulse {span}", switch.name())?
            }
            _ => writeln!(out, "second end: not found")?,
        }
        match self.span {
            Some(span) => {
                let (mm, um) = millimetres(span);
                writeln!(out, "span:       {span} pulses, {mm}.{um:03} mm")?;
            }
            None => writeln!(out, "span:       not measured")?,
        }
        match (self.homed, self.span) {
            (true, Some(span)) => writeln!(out, "position:   pulse {} of {span}", self.position)?,
            _ => writeln!(out, "position:   not homed: press Calibrate")?,
        }
        let Some((action, outcome)) = self.last else {
            return writeln!(out, "last:       nothing yet");
        };
        write!(out, "last:       {}: ", action.label())?;
        match outcome {
            Outcome::Centred { position } => writeln!(out, "centred at pulse {position}; holding"),
            Outcome::Jogged { position, at_switch: None } => writeln!(out, "jogged to pulse {position}"),
            Outcome::Jogged { position, at_switch: Some(switch) } => {
                writeln!(out, "stopped at pulse {position}: {} tripped", switch.name())
            }
            Outcome::Measured(JumpLimit::Measured(m)) => writeln!(out, "m = {m} pulses/s², measured"),
            Outcome::Measured(JumpLimit::AtLeast(m)) => writeln!(out, "m = at least {m} pulses/s²"),
            Outcome::Measured(JumpLimit::Below(m)) => writeln!(out, "m is below j_start, {m} pulses/s²"),
            Outcome::Measured(JumpLimit::Incomplete) => writeln!(out, "not every kind finished"),
            Outcome::Enabled => writeln!(out, "driver enabled"),
            Outcome::Disabled => writeln!(out, "driver disabled; the cart is free and must be homed again"),
            Outcome::Halted(halt) => {
                if halt.frees_cart() {
                    write!(out, "HALTED, driver off: ")?;
                }
                halt.describe(out)?;
                writeln!(out)
            }
        }
    }
}

/// `pulses` of belt as whole mm and the µm left over: µm = pulses ×
/// belt_um_per_rev ÷ pulses_per_rev ([drive]; 15 µm a pulse on this rig).
pub fn millimetres(pulses: u32) -> (u64, u64) {
    let um = u64::from(pulses) * u64::from(BELT_UM_PER_REV) / u64::from(PULSES_PER_REV);
    (um / 1_000, um % 1_000)
}
