//! `config/stepper.toml`: the settings the motor driver's pins are built with.

use crate::toml_value;

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

/// The settings file in `config/`, and the file written into `OUT_DIR`, which
/// `src/config/stepper.rs` includes by the same name.
const FILE_NAME: &str = "stepper.toml";
const SOURCE_FILE_NAME: &str = "stepper.rs";

/// The keys, and the values `dir_positive` accepts.
const DIR_POSITIVE_KEY: &str = "dir_positive";
const DIR_POSITIVE_HIGH: &str = "high";
const DIR_POSITIVE_LOW: &str = "low";
const PULSE_LOW_KEY: &str = "pulse_low_ns";

/// The constants' type for a level, and its two variants, which the key's
/// values become.
const PIN_STATE_TYPE: &str = "embedded_hal::digital::PinState";
const PIN_STATE_HIGH: &str = "High";
const PIN_STATE_LOW: &str = "Low";

/// Reads `config/stepper.toml` and writes its constants into `out_dir`.
pub fn generate(config_dir: &Path, out_dir: &Path) -> Result<()> {
    let path = config_dir.join(FILE_NAME);
    let table = toml_value::read(&path)?;

    let level = match toml_value::text(&table, DIR_POSITIVE_KEY, &path)? {
        DIR_POSITIVE_HIGH => PIN_STATE_HIGH,
        DIR_POSITIVE_LOW => PIN_STATE_LOW,
        other => bail!(
            "{}: {DIR_POSITIVE_KEY} is {other:?}; it must be \"{DIR_POSITIVE_HIGH}\" or \"{DIR_POSITIVE_LOW}\"",
            path.display()
        ),
    };

    let pulse_low_ns = toml_value::whole(&table, PULSE_LOW_KEY, &path)?;

    let source = out_dir.join(SOURCE_FILE_NAME);
    fs::write(
        &source,
        format!(
            "/// The level on D5 for a positive command: `{DIR_POSITIVE_KEY}`.\n\
             pub const DIR_POSITIVE: {PIN_STATE_TYPE} = {PIN_STATE_TYPE}::{level};\n\
             /// STEP's on time, D4 low, ns: `{PULSE_LOW_KEY}`.\n\
             pub const PULSE_LOW_NS: u32 = {pulse_low_ns};\n"
        ),
    )
    .with_context(|| format!("writing {}", source.display()))
}
