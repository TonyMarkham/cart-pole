use clap::Parser;
use std::path::PathBuf;

/// The bench's apply test: times each command from its arrival on the R4 to
/// its new rate on STEP.
#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// The settings file, `apply-host.toml`.
    pub(crate) settings: PathBuf,
}
