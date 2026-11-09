use clap::Subcommand;
use std::path::PathBuf;

/// The tests the bench runs.
#[derive(Debug, Subcommand)]
pub(crate) enum Test {
    /// How long the R4's processor stays awake after each wake, timed by the
    /// R3 on D3, while the R4 follows random commands over WiFi.
    Awake {
        /// The settings file, `bench-host.toml`.
        settings: PathBuf,
    },
}
