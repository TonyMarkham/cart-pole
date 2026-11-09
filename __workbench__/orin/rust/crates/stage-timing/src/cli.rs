use clap::Parser;
use std::path::PathBuf;

/// Gaps 2–6 on the Orin: times each stage of the brain's loop, from the frame
/// arriving to its command being sent.
#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// The settings file, `stage-timing.toml`.
    pub(crate) settings: PathBuf,
}
