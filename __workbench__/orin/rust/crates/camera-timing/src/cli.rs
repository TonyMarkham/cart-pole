use clap::Parser;
use std::path::PathBuf;

/// Gap 1 on the Orin: times how long the camera takes from exposing a frame
/// to the frame being read, by flashing an LED in its view.
#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// The settings file, `camera-timing.toml`.
    pub(crate) settings: PathBuf,
}
