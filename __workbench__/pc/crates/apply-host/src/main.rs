//! The PC side of the bench's apply test (`__workbench__/README.md`, *The
//! apply test*; `real-timings.md`, gap 8): how long the bench R4 takes from
//! having a command whole to running at its new rate, timed by the R3 on
//! one clock, its Timer1. The PC only sends the commands and collects the
//! R3's results; it times nothing itself.
//!
//! `apply-host apply-host.toml`

mod apply_report;
mod apply_result;
mod apply_run;
mod cli;
mod protocol;
mod r4_address;
mod random;
mod result_kind;
mod run_folder;
mod serial_line;
mod settings;
mod stats;
#[cfg(test)]
mod tests;
mod units;

// ---------------------------------------------------------------------------------------------- //

use crate::{cli::Cli, settings::host_settings::HostSettings};

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    apply_run::run(&HostSettings::read(&cli.settings)?, &cli.settings)
}
