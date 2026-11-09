//! Reading a settings file in `config/`, and the values in it.

use anyhow::{Context, Result};
use std::{fs, path::Path};

/// The settings file at `path`. Cargo runs the build script again whenever
/// it changes.
pub fn read(path: &Path) -> Result<toml::Table> {
    println!("cargo:rerun-if-changed={}", path.display());
    fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?
        .parse()
        .with_context(|| format!("parsing {}", path.display()))
}

/// `key`, a whole number from 1 to `u32::MAX`: every number here is a speed, a
/// time, a count or a port, and none means anything at 0.
pub fn whole(table: &toml::Table, key: &str, path: &Path) -> Result<u32> {
    table
        .get(key)
        .and_then(|value| value.as_integer())
        .and_then(|value| u32::try_from(value).ok())
        .filter(|&value| value > 0)
        .with_context(|| {
            format!(
                "{}: {key} must be a whole number from 1 to {}",
                path.display(),
                u32::MAX
            )
        })
}

/// `key`, a string.
pub fn text<'a>(table: &'a toml::Table, key: &str, path: &Path) -> Result<&'a str> {
    table
        .get(key)
        .and_then(|value| value.as_str())
        .with_context(|| format!("{}: {key} must be a string", path.display()))
}

/// Printable ASCII: no control characters can reach the chip's AT parser.
pub fn printable(text: &str) -> bool {
    text.bytes().all(|byte| (b' '..=b'~').contains(&byte))
}
