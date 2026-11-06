//! Feeds `memory.x` to the linker, so the image starts after the Arduino
//! bootloader and uploads over USB keep working, and compiles `config.toml`
//! into the constants the firmware runs on: the board has no file system.

use anyhow::{Context, Result, bail};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// The environment variables cargo gives a build script: the crate's folder
/// (where `Cargo.toml` is) and the folder for its generated files.
const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
const OUT_DIR: &str = "OUT_DIR";

/// The linker script naming the board's flash and RAM, beside `Cargo.toml`.
const MEMORY_FILE_NAME: &str = "memory.x";

/// What the linker is told: `--nmagic` keeps the sections unpadded, and
/// `link.x` is cortex-m-rt's linker script, which includes `memory.x`.
const LINK_ARG_NMAGIC: &str = "--nmagic";
const LINK_ARG_SCRIPT: &str = "-Tlink.x";

/// The settings file, beside `Cargo.toml`.
const CONFIG_FILE_NAME: &str = "config.toml";

/// The Rust file written into `OUT_DIR`. `src/config.rs` includes it by the
/// same name, written out there: `include!` takes only a literal.
const CONFIG_SOURCE_FILE_NAME: &str = "config.rs";

// config.toml
const FIRMWARE_TABLE: &str = "firmware";
const DIR_POSITIVE_KEY: &str = "dir_positive";
const DIR_POSITIVE_HIGH: &str = "high";
const DIR_POSITIVE_LOW: &str = "low";
const PULSE_LOW_KEY: &str = "pulse_low_ns";
const PIN_STATE_HIGH: &str = "High";
const PIN_STATE_LOW: &str = "Low";
const USB_TABLE: &str = "usb";
const USB_BAUD_KEY: &str = "baud";

// config.rs
const PULSE_LOW_CONST: &str = "PULSE_LOW_NS";
const DIR_POSITIVE_CONST: &str = "DIR_POSITIVE";
const USB_BAUD_CONST: &str = "USB_BAUD";
const PIN_STATE_TYPE: &str = "embedded_hal::digital::PinState";

// Standard Arduino Reset baud's
/// The two rates at which the board's USB bridge resets the processor instead
/// of only setting the speed (arduino-manager.toml, [serial] baud, citing the
/// bridge's UNOR4USBBridge.ino, usbEventCallback).
const RESET_BAUDS: [u32; 2] = [1_200, 2_400];

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var(CARGO_MANIFEST_DIR)?);
    let out_dir = PathBuf::from(env::var(OUT_DIR)?);

    // memory.x beside cortex-m-rt's link.x, on the linker's search path.
    let memory_x = manifest_dir.join(MEMORY_FILE_NAME);
    println!("cargo:rerun-if-changed={}", memory_x.display());
    fs::copy(&memory_x, out_dir.join(MEMORY_FILE_NAME))
        .with_context(|| format!("copying {}", memory_x.display()))?;
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-arg-bins={LINK_ARG_NMAGIC}");
    println!("cargo:rustc-link-arg-bins={LINK_ARG_SCRIPT}");

    config(&manifest_dir, &out_dir)
}

/// Reads `config.toml` and writes `config.rs` into `out_dir`, which
/// `src/config.rs` includes: one constant per setting.
fn config(manifest_dir: &Path, out_dir: &Path) -> Result<()> {
    let config = manifest_dir.join(CONFIG_FILE_NAME);
    println!("cargo:rerun-if-changed={}", config.display());
    let table: toml::Table = fs::read_to_string(&config)
        .with_context(|| format!("reading {}", config.display()))?
        .parse()
        .with_context(|| format!("parsing {}", config.display()))?;

    // Configure Stepper Motor's DIR pin state
    let level = table
        .get(FIRMWARE_TABLE)
        .and_then(|firmware| firmware.get(DIR_POSITIVE_KEY))
        .and_then(|value| value.as_str());
    let level = match level {
        Some(DIR_POSITIVE_HIGH) => PIN_STATE_HIGH,
        Some(DIR_POSITIVE_LOW) => PIN_STATE_LOW,
        other => bail!(
            "{}: [{FIRMWARE_TABLE}] {DIR_POSITIVE_KEY} is {other:?}; it must be \"{DIR_POSITIVE_HIGH}\" or \"{DIR_POSITIVE_LOW}\"",
            config.display()
        ),
    };

    // Configure Stepper Motor's PUL pin
    let pulse_low_ns = table
        .get(FIRMWARE_TABLE)
        .and_then(|firmware| firmware.get(PULSE_LOW_KEY))
        .and_then(|value| value.as_integer())
        .and_then(|ns| u32::try_from(ns).ok());
    let Some(pulse_low_ns) = pulse_low_ns else {
        bail!(
            "{}: [{FIRMWARE_TABLE}] {PULSE_LOW_KEY} must be a whole number of ns",
            config.display()
        );
    };

    // Configure USB
    let usb_baud = table
        .get(USB_TABLE)
        .and_then(|usb| usb.get(USB_BAUD_KEY))
        .and_then(|value| value.as_integer())
        .and_then(|baud| u32::try_from(baud).ok())
        .filter(|&baud| baud > 0);
    let Some(usb_baud) = usb_baud else {
        bail!(
            "{}: [{USB_TABLE}] {USB_BAUD_KEY} must be a whole number of bits per second, above 0",
            config.display()
        );
    };
    if RESET_BAUDS.contains(&usb_baud) {
        bail!(
            "{}: [{USB_TABLE}] {USB_BAUD_KEY} is {usb_baud}: opening the port at that rate resets the board",
            config.display()
        );
    }

    // Build config.rs
    let source = out_dir.join(CONFIG_SOURCE_FILE_NAME);
    fs::write(
        &source,
        format!(
            "/// The level on D5 for a positive command: `{DIR_POSITIVE_KEY}`.\n\
             pub const {DIR_POSITIVE_CONST}: {PIN_STATE_TYPE} = {PIN_STATE_TYPE}::{level};\n\
             /// STEP's on time, D4 low, ns: `{PULSE_LOW_KEY}`.\n\
             pub const {PULSE_LOW_CONST}: u32 = {pulse_low_ns};\n\
             /// The USB serial line's speed, bits per second: `[{USB_TABLE}] {USB_BAUD_KEY}`.\n\
             pub const {USB_BAUD_CONST}: u32 = {usb_baud};\n"
        ),
    )
    .with_context(|| format!("writing {}", source.display()))
}
