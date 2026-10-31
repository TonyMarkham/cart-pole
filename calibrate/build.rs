//! Feeds `memory.x` to the linker and compiles `calibrate.toml` (and the
//! WiFi credentials file it names) into the constants the firmware runs
//! on: the board has no file system, so every tunable value is read here,
//! at build time. A change to any of these files rebuilds the image.

use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};

/// The DM542's top pulse rate, pulses/s (Leadshine DM542 manual V1.0,
/// section 2, via control-design.md).
const DRIVER_MAX_RATE: i64 = 200_000;

/// The DM542's timing minima, ns (the same manual, section 10): a pulse
/// at least 2.5 µs on (t3) and 2.5 µs off (t4), DIR settled 5 µs before
/// it (t2).
const DRIVER_MIN_PULSE_NS: i64 = 2_500;
const DRIVER_MIN_GAP_NS: i64 = 2_500;
const DRIVER_MIN_DIR_SETUP_NS: i64 = 5_000;

/// The most pieces a trial's plan holds: `MAX_SEGMENTS` in
/// src/profile.rs. A staircase takes one per step plus five.
const PROFILE_SEGMENTS: i64 = 48;

/// The keys each table of calibrate.toml holds; any other is refused,
/// since nothing would read it.
const CALIBRATE_KEYS: [&str; 15] = [
    "drive_settings",
    "run_at_start",
    "start_ms",
    "seek_speed",
    "home_speed",
    "accel",
    "seek_timeout_ms",
    "jog_pulses",
    "release_pulses",
    "trip_us",
    "stop_check_ms",
    "settle_ms",
    "pulse_low_ns",
    "dir_setup_ns",
    "monitor_baud",
];
const WIFI_KEYS: [&str; 3] = ["credentials", "hostname", "port"];
const REPORT_KEYS: [&str; 2] = ["host", "port"];
const EXPERIMENT_KEYS: [&str; 11] = [
    "conditions",
    "trial_top",
    "trial_dt_ms",
    "j_start",
    "j_growth_percent",
    "trials_per_j",
    "noise_repeats",
    "noise_away_pulses",
    "steady_ms",
    "hold_ms",
    "end_margin_pulses",
];
const CREDENTIAL_KEYS: [&str; 2] = ["ssid", "password"];

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    // memory.x beside cortex-m-rt's link.x, on the linker's search path.
    let memory_x = manifest_dir.join("memory.x");
    println!("cargo:rerun-if-changed={}", memory_x.display());
    fs::copy(&memory_x, out_dir.join("memory.x"))
        .with_context(|| format!("copying {}", memory_x.display()))?;
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");

    let config = manifest_dir.join("calibrate.toml");
    let table = read_table(&config)?;
    if let Some(key) =
        table.keys().find(|key| !["calibrate", "wifi", "experiment", "report"].contains(&key.as_str()))
    {
        bail!(
            "{}: unknown table `{key}`; the tables are [calibrate], [wifi], [experiment] and [report]",
            config.display()
        );
    }
    let calibrate = section(&table, "calibrate", &CALIBRATE_KEYS, &config)?;
    let wifi = section(&table, "wifi", &WIFI_KEYS, &config)?;
    let report = section(&table, "report", &REPORT_KEYS, &config)?;
    let report_host = text(report, "host", &config)?;
    if report_host.parse::<std::net::Ipv4Addr>().is_err() {
        bail!("{}: [report] host `{report_host}` is not an IPv4 address", config.display());
    }
    let report_port = whole(report, "port", &config)?;
    if report_port > 65_535 {
        bail!("{}: [report] port {report_port} is above 65535", config.display());
    }
    let experiment = section(&table, "experiment", &EXPERIMENT_KEYS, &config)?;

    let start_ms = whole(calibrate, "start_ms", &config)?;
    let seek_speed = whole(calibrate, "seek_speed", &config)?;
    let home_speed = whole(calibrate, "home_speed", &config)?;
    let accel = whole(calibrate, "accel", &config)?;
    let seek_timeout_ms = whole(calibrate, "seek_timeout_ms", &config)?;
    let settle_ms = whole(calibrate, "settle_ms", &config)?;
    let monitor_baud = whole(calibrate, "monitor_baud", &config)?;
    let jog_pulses = whole(calibrate, "jog_pulses", &config)?;
    let release_pulses = whole(calibrate, "release_pulses", &config)?;
    let trip_us = whole(calibrate, "trip_us", &config)?;
    let stop_check_ms = whole(calibrate, "stop_check_ms", &config)?;
    let pulse_low_ns = whole(calibrate, "pulse_low_ns", &config)?;
    let dir_setup_ns = whole(calibrate, "dir_setup_ns", &config)?;
    if pulse_low_ns < DRIVER_MIN_PULSE_NS {
        bail!("{}: pulse_low_ns {pulse_low_ns} is under the DM542's {DRIVER_MIN_PULSE_NS}", config.display());
    }
    if dir_setup_ns < DRIVER_MIN_DIR_SETUP_NS {
        bail!("{}: dir_setup_ns {dir_setup_ns} is under the DM542's {DRIVER_MIN_DIR_SETUP_NS}", config.display());
    }
    let run_at_start = calibrate
        .get("run_at_start")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| anyhow!("{}: [calibrate] run_at_start (true or false) is missing", config.display()))?;
    // A seek must have time to leave the switch it starts on.
    if release_pulses * 1_000 / seek_speed >= seek_timeout_ms {
        bail!(
            "{}: release_pulses {release_pulses} take longer at seek_speed than seek_timeout_ms {seek_timeout_ms}",
            config.display()
        );
    }
    if seek_speed > home_speed {
        bail!("{}: seek_speed {seek_speed} is above home_speed {home_speed}", config.display());
    }
    // At the top rate the pulse must still leave the DM542 its gap.
    let gap_ok = |rate: i64| 1_000_000_000 / rate - pulse_low_ns >= DRIVER_MIN_GAP_NS;
    if !gap_ok(home_speed) {
        bail!(
            "{}: at home_speed {home_speed}, pulse_low_ns {pulse_low_ns} leaves under {DRIVER_MIN_GAP_NS} ns off",
            config.display()
        );
    }
    if home_speed > DRIVER_MAX_RATE {
        bail!(
            "{}: home_speed {home_speed} is above the DM542's {DRIVER_MAX_RATE} pulses/s",
            config.display()
        );
    }

    // The experiment (jump-limit-experiment.md).
    let conditions = text(experiment, "conditions", &config)?;
    let trial_top = whole(experiment, "trial_top", &config)?;
    let trial_dt_ms = whole(experiment, "trial_dt_ms", &config)?;
    let j_start = whole(experiment, "j_start", &config)?;
    let j_growth_percent = whole(experiment, "j_growth_percent", &config)?;
    let trials_per_j = whole(experiment, "trials_per_j", &config)?;
    let noise_repeats = whole(experiment, "noise_repeats", &config)?;
    let noise_away_pulses = whole(experiment, "noise_away_pulses", &config)?;
    let steady_ms = whole(experiment, "steady_ms", &config)?;
    let hold_ms = whole(experiment, "hold_ms", &config)?;
    let end_margin_pulses = whole(experiment, "end_margin_pulses", &config)?;
    if trial_top > DRIVER_MAX_RATE || !gap_ok(trial_top) {
        bail!(
            "{}: [experiment] trial_top {trial_top} is above the DM542's {DRIVER_MAX_RATE}, or leaves pulse_low_ns no gap",
            config.display()
        );
    }
    if trial_top * 2 > i64::from(i32::MAX) {
        bail!("{}: [experiment] trial_top {trial_top} is too large", config.display());
    }
    let first_step = j_start * trial_dt_ms / 1_000;
    if first_step < 1 || (trial_top + first_step - 1) / first_step + 5 > PROFILE_SEGMENTS {
        bail!(
            "{}: [experiment] j_start {j_start}: a staircase at it needs more than {PROFILE_SEGMENTS} pieces; raise it",
            config.display()
        );
    }
    if noise_away_pulses < release_pulses {
        bail!(
            "{}: [experiment] noise_away_pulses {noise_away_pulses} is under release_pulses {release_pulses}",
            config.display()
        );
    }

    // The drive figures, from the file drive_settings names.
    let drive_path = manifest_dir.join(text(calibrate, "drive_settings", &config)?);
    let drive_table = read_table(&drive_path)?;
    let drive = drive_table
        .get("drive")
        .and_then(|value| value.as_table())
        .ok_or_else(|| anyhow!("{}: [drive] is missing", drive_path.display()))?;
    let full_steps_per_rev = whole(drive, "full_steps_per_rev", &drive_path)?;
    let microsteps = whole(drive, "microsteps", &drive_path)?;
    let belt_um_per_rev = whole(drive, "belt_um_per_rev", &drive_path)?;
    let pulses_per_rev = full_steps_per_rev * microsteps;
    if pulses_per_rev > i64::from(u32::MAX) {
        bail!("{}: full_steps_per_rev × microsteps = {pulses_per_rev} does not fit a u32", drive_path.display());
    }

    // WiFi: the hostname and port from calibrate.toml, the network from
    // the credentials file (kept out of git).
    let hostname = text(wifi, "hostname", &config)?;
    if hostname.is_empty()
        || hostname.len() > 31
        || !hostname.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!(
            "{}: [wifi] hostname `{hostname}`: letters, digits and hyphens, 1 to 31 of them",
            config.display()
        );
    }
    let port = whole(wifi, "port", &config)?;
    if port > 65_535 {
        bail!("{}: [wifi] port {port} is above 65535", config.display());
    }
    let credentials_path = manifest_dir.join(text(wifi, "credentials", &config)?);
    if !credentials_path.exists() {
        bail!(
            "{} is missing: copy wifi.example.toml to that name and fill in the network",
            credentials_path.display()
        );
    }
    let credentials = read_table(&credentials_path)?;
    if let Some(key) = credentials.keys().find(|key| !CREDENTIAL_KEYS.contains(&key.as_str())) {
        bail!(
            "{}: unknown key {key}; the keys are {}",
            credentials_path.display(),
            CREDENTIAL_KEYS.join(", ")
        );
    }
    let ssid = text(&credentials, "ssid", &credentials_path)?;
    let password = text(&credentials, "password", &credentials_path)?;
    // WPA2's limits: an SSID of 1 to 32 bytes, a passphrase of 8 to 63
    // printable characters (or none, for an open network).
    if ssid.is_empty() || ssid.len() > 32 || !printable(ssid) {
        bail!("{}: ssid must be 1 to 32 printable ASCII characters", credentials_path.display());
    }
    if !(password.is_empty() || (8..=63).contains(&password.len())) || !printable(password) {
        bail!(
            "{}: password must be empty or 8 to 63 printable ASCII characters",
            credentials_path.display()
        );
    }

    let mut out = String::new();
    writeln!(out, "// Generated by build.rs from {}.", config.display())?;
    constant(&mut out, "Wait after power-up before moving, ms: `start_ms`.", "START_MS", start_ms)?;
    constant(&mut out, "The seek toward each switch, and jogs, pulses/s: `seek_speed`.", "SEEK_SPEED", seek_speed)?;
    constant(&mut out, "The top of the move to the centre, pulses/s: `home_speed`.", "HOME_SPEED", home_speed)?;
    constant(&mut out, "The ramp's acceleration, pulses/s²: `accel`.", "ACCEL", accel)?;
    constant(&mut out, "The longest one seek may run, ms: `seek_timeout_ms`.", "SEEK_TIMEOUT_MS", seek_timeout_ms)?;
    constant(&mut out, "The pause at each switch, ms: `settle_ms`.", "SETTLE_MS", settle_ms)?;
    constant(&mut out, "The monitor line's baud: `monitor_baud`.", "MONITOR_BAUD", monitor_baud)?;
    writeln!(out, "/// Whether power-up homes the cart by itself: `run_at_start`.")?;
    writeln!(out, "pub const RUN_AT_START: bool = {run_at_start};")?;
    constant(&mut out, "One jog's pulses, at `seek_speed`: `jog_pulses`.", "JOG_PULSES", jog_pulses)?;
    constant(
        &mut out,
        "Pulses a switch the cart starts on must read released before it counts: `release_pulses`.",
        "RELEASE_PULSES",
        release_pulses,
    )?;
    constant(&mut out, "The STOP check's period during slow moves, ms: `stop_check_ms`.", "STOP_CHECK_MS", stop_check_ms)?;
    constant(&mut out, "How long a switch must read pressed to count, µs: `trip_us`.", "TRIP_US", trip_us)?;
    constant(&mut out, "STEP's on time, D4 low, ns: `pulse_low_ns`.", "PULSE_LOW_NS", pulse_low_ns)?;
    constant(&mut out, "DIR's setup before a pulse, ns: `dir_setup_ns`.", "DIR_SETUP_NS", dir_setup_ns)?;
    text_constant(&mut out, "The rig's conditions, for the experiment's report: `conditions`.", "CONDITIONS", conditions)?;
    constant(&mut out, "The experiment's top rate, pulses/s: `trial_top`.", "TRIAL_TOP", trial_top)?;
    constant(&mut out, "The gap between commands, ms: `trial_dt_ms`.", "TRIAL_DT_MS", trial_dt_ms)?;
    constant(&mut out, "The first change rate tried, pulses/s²: `j_start`.", "J_START", j_start)?;
    constant(&mut out, "How much j rises a step, %: `j_growth_percent`.", "J_GROWTH_PERCENT", j_growth_percent)?;
    constant(&mut out, "Trials at each j: `trials_per_j`.", "TRIALS_PER_J", trials_per_j)?;
    constant(&mut out, "Returns to the switch for the noise floor: `noise_repeats`.", "NOISE_REPEATS", noise_repeats)?;
    constant(&mut out, "How far each noise-floor return starts, pulses: `noise_away_pulses`.", "NOISE_AWAY_PULSES", noise_away_pulses)?;
    constant(&mut out, "The hold at c_a before the jump, ms: `steady_ms`.", "STEADY_MS", steady_ms)?;
    constant(&mut out, "The hold at c_b after the jump, ms: `hold_ms`.", "HOLD_MS", hold_ms)?;
    constant(&mut out, "The room a trial leaves at each end, pulses: `end_margin_pulses`.", "END_MARGIN_PULSES", end_margin_pulses)?;
    constant(&mut out, "Belt per pulley turn, µm: [drive] `belt_um_per_rev`.", "BELT_UM_PER_REV", belt_um_per_rev)?;
    constant(
        &mut out,
        "Pulses per pulley turn: [drive] `full_steps_per_rev` × `microsteps`.",
        "PULSES_PER_REV",
        pulses_per_rev,
    )?;
    constant(&mut out, "The page's TCP port: [wifi] `port`.", "HTTP_PORT", port)?;
    text_constant(&mut out, "The bench station's address, for trial records: [report] `host`.", "REPORT_HOST", report_host)?;
    constant(&mut out, "The bench station's port: [report] `port`.", "REPORT_PORT", report_port)?;
    text_constant(&mut out, "The network's name, for messages: `ssid`.", "WIFI_SSID", ssid)?;
    text_constant(
        &mut out,
        "Stores the name the router lists: [wifi] `hostname`.",
        "AT_HOSTNAME",
        &format!("AT+HOSTNAME=\"{hostname}\"\r\n"),
    )?;
    // Quoted, so commas and spaces survive the bridge's parser, which
    // splits on unquoted commas and takes `\` as an escape
    // (arduino-embedded-rust WiFi.md, *The wire protocol*).
    text_constant(
        &mut out,
        "Joins the network in the credentials file.",
        "AT_BEGINSTA",
        &format!("AT+BEGINSTA=\"{}\",\"{}\"\r\n", at_quoted(ssid), at_quoted(password)),
    )?;
    text_constant(
        &mut out,
        "Opens the TCP server on [wifi] `port`.",
        "AT_BEGINSERVER",
        &format!("AT+BEGINSERVER={port}\r\n"),
    )?;
    fs::write(out_dir.join("settings.rs"), out)?;
    Ok(())
}

fn read_table(path: &Path) -> Result<toml::Table> {
    println!("cargo:rerun-if-changed={}", path.display());
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    text.parse().with_context(|| format!("parsing {}", path.display()))
}

/// The table `name`, holding only `keys`.
fn section<'a>(table: &'a toml::Table, name: &str, keys: &[&str], path: &Path) -> Result<&'a toml::Table> {
    let section = table
        .get(name)
        .and_then(|value| value.as_table())
        .ok_or_else(|| anyhow!("{}: [{name}] is missing", path.display()))?;
    if let Some(key) = section.keys().find(|key| !keys.contains(&key.as_str())) {
        bail!("{}: unknown key [{name}] {key}; the keys are {}", path.display(), keys.join(", "));
    }
    Ok(section)
}

/// A whole number from 1 to `u32::MAX`: every number here is a count,
/// a rate, a time or a port, and none is meaningful at 0.
fn whole(table: &toml::Table, key: &str, path: &Path) -> Result<i64> {
    let value = table
        .get(key)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| anyhow!("{}: {key} (a whole number) is missing", path.display()))?;
    if !(1..=i64::from(u32::MAX)).contains(&value) {
        bail!("{}: {key} = {value}; it must be from 1 to {}", path.display(), u32::MAX);
    }
    Ok(value)
}

fn text<'a>(table: &'a toml::Table, key: &str, path: &Path) -> Result<&'a str> {
    table
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("{}: {key} (a string) is missing", path.display()))
}

fn printable(text: &str) -> bool {
    text.bytes().all(|byte| (b' '..=b'~').contains(&byte))
}

/// `text` for inside an AT argument's quotes: `"` and `\` escaped.
fn at_quoted(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '"' || character == '\\' {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted
}

fn constant(out: &mut String, doc: &str, name: &str, value: i64) -> Result<()> {
    writeln!(out, "/// {doc}")?;
    writeln!(out, "pub const {name}: u32 = {value};")?;
    Ok(())
}

/// A `&str` constant; `{:?}` writes it as an escaped Rust literal.
fn text_constant(out: &mut String, doc: &str, name: &str, value: &str) -> Result<()> {
    writeln!(out, "/// {doc}")?;
    writeln!(out, "pub const {name}: &str = {value:?};")?;
    Ok(())
}
