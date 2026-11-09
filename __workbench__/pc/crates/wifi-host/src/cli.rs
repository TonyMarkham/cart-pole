use clap::Parser;
use std::path::PathBuf;

/// The bench's WiFi test: times the hop from this PC to the bench R4.
#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// The settings file, `wifi-host.toml`.
    pub(crate) settings: PathBuf,
}
