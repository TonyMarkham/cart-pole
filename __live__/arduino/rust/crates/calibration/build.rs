//! Feeds `memory.x` to the linker, so the image starts after the Arduino
//! bootloader and uploads over USB keep working, and compiles
//! `calibration.toml` (and the WiFi credentials file it names) into the
//! constants the firmware runs on: the board has no file system.

use anyhow::{Context, Result, anyhow, bail};
use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

const FIRMWARE_KEYS: [&str; 9] = [
    "monitor_baud",
    "centre_speed_max",
    "at_baud",
    "seek_speed",
    "seek_timeout_ms",
    "release_pulses",
    "trip_us",
    "pulse_low_ns",
    "dir_setup_ns",
];
const END_KEYS: [&str; 2] = ["switch", "dir"];

/// `[timing]`: each key, the constant it becomes, and that constant's doc.
const TIMING: [(&str, &str, &str); 15] = [
    (
        "poll_us",
        "POLL_US",
        "Between looks at the receive ring, µs: `poll_us`.",
    ),
    (
        "frame_timeout_ms",
        "FRAME_TIMEOUT_MS",
        "A datagram's payload, ms: `frame_timeout_ms`.",
    ),
    (
        "drain_quiet_ms",
        "DRAIN_QUIET_MS",
        "Quiet that ends a drain, ms: `drain_quiet_ms`.",
    ),
    (
        "reply_timeout_ms",
        "REPLY_TIMEOUT_MS",
        "A command's reply, ms: `reply_timeout_ms`.",
    ),
    (
        "handshake_tries",
        "HANDSHAKE_TRIES",
        "Tries of the first `AT`: `handshake_tries`.",
    ),
    (
        "handshake_timeout_ms",
        "HANDSHAKE_TIMEOUT_MS",
        "Each `AT`'s reply, ms: `handshake_timeout_ms`.",
    ),
    (
        "handshake_pause_ms",
        "HANDSHAKE_PAUSE_MS",
        "Between `AT` tries, ms: `handshake_pause_ms`.",
    ),
    (
        "soft_reset_tries",
        "SOFT_RESET_TRIES",
        "Tries of clearing WiFi state: `soft_reset_tries`.",
    ),
    (
        "soft_reset_timeout_ms",
        "SOFT_RESET_TIMEOUT_MS",
        "Each clearing's reply, ms: `soft_reset_timeout_ms`.",
    ),
    (
        "wifi_mode_timeout_ms",
        "WIFI_MODE_TIMEOUT_MS",
        "A WiFi mode change's reply, ms: `wifi_mode_timeout_ms`.",
    ),
    (
        "begin_sta_timeout_ms",
        "BEGIN_STA_TIMEOUT_MS",
        "Starting the join's reply, ms: `begin_sta_timeout_ms`.",
    ),
    (
        "status_tries",
        "STATUS_TRIES",
        "Asks whether the join finished: `status_tries`.",
    ),
    (
        "status_pause_ms",
        "STATUS_PAUSE_MS",
        "Before each of those asks, ms: `status_pause_ms`.",
    ),
    (
        "address_tries",
        "ADDRESS_TRIES",
        "Asks for the router's address: `address_tries`.",
    ),
    (
        "address_pause_ms",
        "ADDRESS_PAUSE_MS",
        "Between those asks, ms: `address_pause_ms`.",
    ),
];

/// The DM542's shortest STEP on and off times, ns (manual, section 10, t3
/// and t4), and its shortest DIR setup (t2).
const DM542_STEP_MIN_NS: i64 = 2_500;
const DM542_DIR_SETUP_MIN_NS: i64 = 5_000;

const WIFI_KEYS: [&str; 3] = ["credentials", "hostname", "udp_port"];
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

    let config = manifest_dir.join("calibration.toml");
    let table = read_table(&config)?;
    if let Some(key) = table
        .keys()
        .find(|key| !["firmware", "idler", "motor", "timing", "wifi"].contains(&key.as_str()))
    {
        bail!(
            "{}: unknown table `{key}`; the tables are [firmware], [idler], [motor], [timing] and [wifi]",
            config.display()
        );
    }
    let firmware = section(&table, "firmware", &FIRMWARE_KEYS, &config)?;
    let timing = section(&table, "timing", &TIMING.map(|(key, _, _)| key), &config)?;
    let wifi = section(&table, "wifi", &WIFI_KEYS, &config)?;

    let monitor_baud = whole(firmware, "monitor_baud", &config)?;
    let at_baud = whole(firmware, "at_baud", &config)?;
    let seek_speed = whole(firmware, "seek_speed", &config)?;
    let seek_timeout_ms = whole(firmware, "seek_timeout_ms", &config)?;
    let release_pulses = whole(firmware, "release_pulses", &config)?;
    let trip_us = whole(firmware, "trip_us", &config)?;
    let pulse_low_ns = whole(firmware, "pulse_low_ns", &config)?;
    let dir_setup_ns = whole(firmware, "dir_setup_ns", &config)?;

    let seek_period_ns = 1_000_000_000 / seek_speed;
    if pulse_low_ns < DM542_STEP_MIN_NS || seek_period_ns - pulse_low_ns < DM542_STEP_MIN_NS {
        bail!(
            "{}: pulse_low_ns {pulse_low_ns} at seek_speed {seek_speed}: the DM542 needs STEP on and off for at least {DM542_STEP_MIN_NS} ns each",
            config.display()
        );
    }
    if dir_setup_ns < DM542_DIR_SETUP_MIN_NS {
        bail!(
            "{}: dir_setup_ns {dir_setup_ns}: the DM542 needs at least {DM542_DIR_SETUP_MIN_NS}",
            config.display()
        );
    }

    let centre_speed_max = whole(firmware, "centre_speed_max", &config)?;
    if 1_000_000_000 / centre_speed_max - pulse_low_ns < DM542_STEP_MIN_NS {
        bail!(
            "{}: pulse_low_ns {pulse_low_ns} at centre_speed_max {centre_speed_max}: the DM542 needs STEP off for at least {DM542_STEP_MIN_NS} ns",
            config.display()
        );
    }

    let idler = rail_end(&table, "idler", &config)?;
    let motor = rail_end(&table, "motor", &config)?;
    if idler.0 == motor.0 {
        bail!(
            "{}: [idler] and [motor] name the same switch",
            config.display()
        );
    }

    let hostname = text(wifi, "hostname", &config)?;
    if hostname.is_empty()
        || hostname.len() > 31
        || !hostname
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!(
            "{}: [wifi] hostname `{hostname}`: letters, digits and hyphens, 1 to 31 of them",
            config.display()
        );
    }
    let port = whole(wifi, "udp_port", &config)?;
    if port > 65_535 {
        bail!(
            "{}: [wifi] udp_port {port} is above 65535",
            config.display()
        );
    }

    let credentials_path = manifest_dir.join(text(wifi, "credentials", &config)?);
    if !credentials_path.exists() {
        bail!(
            "{} is missing: copy wifi.example.toml to that name and fill in the network",
            credentials_path.display()
        );
    }
    let credentials = read_table(&credentials_path)?;
    if let Some(key) = credentials
        .keys()
        .find(|key| !CREDENTIAL_KEYS.contains(&key.as_str()))
    {
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
        bail!(
            "{}: ssid must be 1 to 32 printable ASCII characters",
            credentials_path.display()
        );
    }
    if !(password.is_empty() || (8..=63).contains(&password.len())) || !printable(password) {
        bail!(
            "{}: password must be empty or 8 to 63 printable ASCII characters",
            credentials_path.display()
        );
    }

    let mut out = String::new();
    writeln!(out, "// Generated by build.rs from {}.", config.display())?;
    constant(
        &mut out,
        "The USB serial port's speed: `monitor_baud`.",
        "MONITOR_BAUD",
        monitor_baud,
    )?;
    constant(
        &mut out,
        "The link to the WiFi chip's speed: `at_baud`.",
        "AT_BAUD",
        at_baud,
    )?;
    constant(
        &mut out,
        "One pulse's period at `seek_speed`, ns. The loop's own work adds to it, so the real rate is a little under the asked one, never over it.",
        "SEEK_PERIOD_NS",
        seek_period_ns,
    )?;
    constant(
        &mut out,
        "The longest one seek may run, ms: `seek_timeout_ms`.",
        "SEEK_TIMEOUT_MS",
        seek_timeout_ms,
    )?;
    constant(
        &mut out,
        "Pulses a starting switch must read released: `release_pulses`.",
        "RELEASE_PULSES",
        release_pulses,
    )?;
    constant(
        &mut out,
        "How long a switch must read pressed, µs: `trip_us`.",
        "TRIP_US",
        trip_us,
    )?;
    constant(
        &mut out,
        "STEP's on time, ns: `pulse_low_ns`.",
        "PULSE_LOW_NS",
        pulse_low_ns,
    )?;
    constant(
        &mut out,
        "DIR's setup before a pulse, ns: `dir_setup_ns`.",
        "DIR_SETUP_NS",
        dir_setup_ns,
    )?;
    constant(
        &mut out,
        "The fastest `MoveToCentre` may ask for, pulses/s: `centre_speed_max`.",
        "CENTRE_SPEED_MAX",
        centre_speed_max,
    )?;
    for (key, name, doc) in TIMING {
        constant(&mut out, doc, name, whole(timing, key, &config)?)?;
    }
    // Worked out here, so the message cannot disagree with the two settings.
    let join_limit_ms =
        whole(timing, "status_tries", &config)? * whole(timing, "status_pause_ms", &config)?;
    text_constant(
        &mut out,
        "What the serial port says when the join runs out of tries.",
        "JOIN_TIMED_OUT",
        &format!("the join did not finish within {join_limit_ms} ms"),
    )?;
    end_constant(
        &mut out,
        "Where `CalibrateIdlerTravelLimit` goes: [idler].",
        "IDLER_END",
        idler,
    )?;
    end_constant(
        &mut out,
        "Where `CalibrateMotorTravelLimit` goes: [motor].",
        "MOTOR_END",
        motor,
    )?;
    constant(
        &mut out,
        "The UDP port the board listens on: [wifi] `udp_port`.",
        "UDP_PORT",
        port,
    )?;
    text_constant(
        &mut out,
        "The network's name, for messages: `ssid`.",
        "WIFI_SSID",
        ssid,
    )?;
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
        &format!(
            "AT+BEGINSTA=\"{}\",\"{}\"\r\n",
            at_quoted(ssid),
            at_quoted(password)
        ),
    )?;
    text_constant(
        &mut out,
        "Opens the UDP socket on [wifi] `udp_port`.",
        "AT_UDPBEGIN",
        &format!("AT+UDPBEGIN={port}\r\n"),
    )?;
    fs::write(out_dir.join("settings.rs"), out)?;
    Ok(())
}

fn read_table(path: &Path) -> Result<toml::Table> {
    println!("cargo:rerun-if-changed={}", path.display());
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    text.parse()
        .with_context(|| format!("parsing {}", path.display()))
}

/// The table `name`, holding only `keys`.
fn section<'a>(
    table: &'a toml::Table,
    name: &str,
    keys: &[&str],
    path: &Path,
) -> Result<&'a toml::Table> {
    let section = table
        .get(name)
        .and_then(|value| value.as_table())
        .ok_or_else(|| anyhow!("{}: [{name}] is missing", path.display()))?;
    if let Some(key) = section.keys().find(|key| !keys.contains(&key.as_str())) {
        bail!(
            "{}: unknown key [{name}] {key}; the keys are {}",
            path.display(),
            keys.join(", ")
        );
    }
    Ok(section)
}

/// A whole number from 1 to `u32::MAX`: every number here is a rate, a
/// time or a port, and none is meaningful at 0.
fn whole(table: &toml::Table, key: &str, path: &Path) -> Result<i64> {
    let value = table
        .get(key)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| anyhow!("{}: {key} (a whole number) is missing", path.display()))?;
    if !(1..=i64::from(u32::MAX)).contains(&value) {
        bail!(
            "{}: {key} = {value}; it must be from 1 to {}",
            path.display(),
            u32::MAX
        );
    }
    Ok(value)
}

fn text<'a>(table: &'a toml::Table, key: &str, path: &Path) -> Result<&'a str> {
    table
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("{}: {key} (a string) is missing", path.display()))
}

/// The table `name` as the Rust names of its switch and direction.
fn rail_end(table: &toml::Table, name: &str, path: &Path) -> Result<(&'static str, &'static str)> {
    let end = section(table, name, &END_KEYS, path)?;
    let switch = match text(end, "switch", path)? {
        "d2" => "D2",
        "d3" => "D3",
        other => bail!(
            "{}: [{name}] switch `{other}`: it must be d2 or d3",
            path.display()
        ),
    };
    let direction = match text(end, "dir", path)? {
        "high" => "High",
        "low" => "Low",
        other => bail!(
            "{}: [{name}] dir `{other}`: it must be high or low",
            path.display()
        ),
    };
    Ok((switch, direction))
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

fn end_constant(
    out: &mut String,
    doc: &str,
    name: &str,
    (switch, direction): (&str, &str),
) -> Result<()> {
    writeln!(out, "/// {doc}")?;
    writeln!(
        out,
        "pub const {name}: crate::end::End = crate::end::End {{ switch: crate::switch::Switch::{switch}, direction: crate::direction::Direction::{direction} }};"
    )?;
    Ok(())
}

/// A `&str` constant; `{:?}` writes it as an escaped Rust literal.
fn text_constant(out: &mut String, doc: &str, name: &str, value: &str) -> Result<()> {
    writeln!(out, "/// {doc}")?;
    writeln!(out, "pub const {name}: &str = {value:?};")?;
    Ok(())
}
