//! `config/calibration.toml`: the settings the calibration's moves run with.

use crate::toml_value;

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

/// The settings file in `config/`, and the file written into `OUT_DIR`, which
/// `src/config/calibration.rs` includes by the same name.
const FILE_NAME: &str = "calibration.toml";
const SOURCE_FILE_NAME: &str = "calibration.rs";

/// The keys.
const SEEK_SPEED_KEY: &str = "seek_speed";
const SEEK_TIMEOUT_KEY: &str = "seek_timeout_ms";
const CENTRE_SPEED_MAX_KEY: &str = "centre_speed_max";
const RELEASE_PULSES_KEY: &str = "release_pulses";

/// Microseconds in a second and in a millisecond, for the time
/// `release_pulses` takes.
const MICROS_PER_SECOND: u64 = 1_000_000;
const MICROS_PER_MILLI: u64 = 1_000;

/// Reads `config/calibration.toml` and writes its constants into `out_dir`.
pub fn generate(config_dir: &Path, out_dir: &Path) -> Result<()> {
    let path = config_dir.join(FILE_NAME);
    let table = toml_value::read(&path)?;

    let seek_speed = rate(&table, SEEK_SPEED_KEY, &path)?;
    let centre_speed_max = rate(&table, CENTRE_SPEED_MAX_KEY, &path)?;
    // A move to the centre with no speed runs at seek_speed, so that must be
    // a speed it may ask for.
    if centre_speed_max < seek_speed {
        bail!(
            "{}: {CENTRE_SPEED_MAX_KEY} is {centre_speed_max}, under {SEEK_SPEED_KEY}'s \
             {seek_speed}",
            path.display()
        );
    }

    let seek_timeout_ms = toml_value::whole(&table, SEEK_TIMEOUT_KEY, &path)?;
    let release_pulses = toml_value::whole(&table, RELEASE_PULSES_KEY, &path)?;

    // A seek runs at one speed, so moving `release_pulses` takes this long,
    // and the seek's time limit must leave room after it.
    let release_us = (u64::from(release_pulses) * MICROS_PER_SECOND)
        .div_ceil(u64::from(seek_speed.unsigned_abs()));
    if release_us >= u64::from(seek_timeout_ms) * MICROS_PER_MILLI {
        bail!(
            "{}: {RELEASE_PULSES_KEY} takes {release_us} µs at {SEEK_SPEED_KEY}, \
             not less than {SEEK_TIMEOUT_KEY}",
            path.display()
        );
    }

    let source = out_dir.join(SOURCE_FILE_NAME);
    fs::write(
        &source,
        format!(
            "/// A seek's speed, pulses per second: `{SEEK_SPEED_KEY}`.\n\
             pub const SEEK_SPEED: i32 = {seek_speed};\n\
             /// The longest one seek runs, ms: `{SEEK_TIMEOUT_KEY}`.\n\
             pub const SEEK_TIMEOUT_MS: u32 = {seek_timeout_ms};\n\
             /// The fastest move to the centre, pulses per second: `{CENTRE_SPEED_MAX_KEY}`.\n\
             pub const CENTRE_SPEED_MAX: i32 = {centre_speed_max};\n\
             /// How far a move goes before a switch it started on counts: `{RELEASE_PULSES_KEY}`.\n\
             pub const RELEASE_PULSES: u32 = {release_pulses};\n"
        ),
    )
        .with_context(|| format!("writing {}", source.display()))
}

/// `key`, a speed in pulses per second. The step generator takes a signed
/// rate, so it must fit an i32.
fn rate(table: &toml::Table, key: &str, path: &Path) -> Result<i32> {
    let speed = toml_value::whole(table, key, path)?;
    let Ok(speed) = i32::try_from(speed) else {
        bail!(
            "{}: {key} is {speed}; a rate is at most {}",
            path.display(),
            i32::MAX
        );
    };
    Ok(speed)
}
