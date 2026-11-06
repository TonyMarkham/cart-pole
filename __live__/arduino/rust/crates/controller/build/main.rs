//! The build script, run on the PC before the firmware compiles. It feeds
//! `memory.x` to the linker, so the image starts after the Arduino bootloader
//! and uploads over USB keep working, and compiles each part's settings in
//! `config/` into the constants that part runs on: the board has no file
//! system.

mod stepper;
mod toml_value;
mod usb;
mod wifi;
mod wifi_timing;

// ---------------------------------------------------------------------------------------------- //

use anyhow::{Context, Result};
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

/// The folder holding each part's settings file, beside `Cargo.toml`.
const CONFIG_DIR: &str = "config";

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var(CARGO_MANIFEST_DIR)?);
    let out_dir = PathBuf::from(env::var(OUT_DIR)?);

    link(&manifest_dir, &out_dir)?;

    let config_dir = manifest_dir.join(CONFIG_DIR);
    stepper::generate(&config_dir, &out_dir)?;
    usb::generate(&config_dir, &out_dir)?;
    wifi::generate(&config_dir, &out_dir)?;
    wifi_timing::generate(&config_dir, &out_dir)?;

    Ok(())
}

/// Puts `memory.x` beside cortex-m-rt's `link.x`, on the linker's search path.
fn link(manifest_dir: &Path, out_dir: &Path) -> Result<()> {
    let memory_x = manifest_dir.join(MEMORY_FILE_NAME);
    println!("cargo:rerun-if-changed={}", memory_x.display());
    fs::copy(&memory_x, out_dir.join(MEMORY_FILE_NAME))
        .with_context(|| format!("copying {}", memory_x.display()))?;
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-arg-bins={LINK_ARG_NMAGIC}");
    println!("cargo:rustc-link-arg-bins={LINK_ARG_SCRIPT}");
    Ok(())
}
