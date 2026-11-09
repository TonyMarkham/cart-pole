//! `config/motion.toml`: the settings the Orin's commands run with.

use crate::toml_value;

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

/// The settings file in `config/`, and the file written into `OUT_DIR`, which
/// `src/config/motion.rs` includes by the same name.
const FILE_NAME: &str = "motion.toml";
const SOURCE_FILE_NAME: &str = "motion.rs";

/// The keys.
const JUMP_LIMIT_KEY: &str = "jump_limit";
const TOP_RATE_KEY: &str = "top_rate";

/// Reads `config/motion.toml` and writes its constants into `out_dir`.
pub fn generate(config_dir: &Path, out_dir: &Path) -> Result<()> {
    let path = config_dir.join(FILE_NAME);
    let table = toml_value::read(&path)?;

    let jump_limit = toml_value::whole(&table, JUMP_LIMIT_KEY, &path)?;

    // The command's size is a u16 (real-loop-dia.md, *Contracts*), so a top
    // rate above 65,535 could never be asked for; the step generator takes a
    // signed rate.
    let top_rate = toml_value::whole(&table, TOP_RATE_KEY, &path)?;
    if top_rate > u32::from(u16::MAX) {
        bail!(
            "{}: {TOP_RATE_KEY} is {top_rate}; a command's size is at most {}",
            path.display(),
            u16::MAX
        );
    }

    let source = out_dir.join(SOURCE_FILE_NAME);
    fs::write(
        &source,
        format!(
            "/// The jump limit m, pulses/s²: `{JUMP_LIMIT_KEY}`.\n\
             pub const JUMP_LIMIT: u32 = {jump_limit};\n\
             /// The top rate every command is clamped to, pulses/s: `{TOP_RATE_KEY}`.\n\
             pub const TOP_RATE: i32 = {top_rate};\n"
        ),
    )
    .with_context(|| format!("writing {}", source.display()))
}
