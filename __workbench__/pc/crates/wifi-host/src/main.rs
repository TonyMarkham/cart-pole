//! The PC side of the bench's WiFi test (`__workbench__/README.md`, *The
//! WiFi test*; `__documentation__/real-timings.md`, gap 7): how long a datagram takes from
//! leaving this PC to the bench R4's processor having it whole, timed on one
//! clock, the PC's.
//!
//! The R4 stamps each datagram on its own clock, which the PC can't read
//! directly. So the PC also syncs the two clocks over USB, before every
//! datagram and in runs at both ends, with NTP's four-stamp exchange; a
//! straight line through the best syncs carries every R4 stamp into the
//! PC's time.
//!
//! `wifi-host wifi-host.toml`

mod analysis;
mod board;
mod cli;
mod clock_fit;
mod hop;
mod ping;
mod probe_run;
mod protocol;
mod r4_address;
mod random;
mod run_folder;
mod serial_line;
mod settings;
mod stats;
mod sync_exchange;
mod sync_point;
#[cfg(test)]
mod tests;
mod units;
mod widener;

// ---------------------------------------------------------------------------------------------- //

use crate::{cli::Cli, settings::host_settings::HostSettings};

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    probe_run::run(&HostSettings::read(&cli.settings)?, &cli.settings)
}
