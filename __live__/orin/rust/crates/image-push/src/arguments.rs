use crate::DEFAULT_SERVER;

use clap::Parser;
use std::path::PathBuf;

/// Push a PNG to the server, which pushes it to every connected viewer.
#[derive(Parser)]
#[command(version)]
pub(crate) struct Args {
    /// PNG file to push
    pub(crate) png: PathBuf,

    /// Base URL of the server
    #[arg(long, default_value = DEFAULT_SERVER)]
    pub(crate) server: String,
}
