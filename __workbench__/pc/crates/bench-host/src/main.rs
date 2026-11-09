//! The PC side of the bench (`__workbench__/README.md`): it drives the bench
//! R4 over WiFi and USB, starts and reads the R3's measurements over USB, and
//! writes each run's results into a folder of its own.
//!
//! `bench-host awake bench-host.toml`: how long the R4's processor stays awake
//! after each tick and each datagram, timed by the R3 on D3.

mod awake;
mod cli;
mod protocol;
mod r4_address;
mod random;
mod run_folder;
mod serial_line;
mod settings;
mod test;
#[cfg(test)]
mod tests;
mod units;

// ---------------------------------------------------------------------------------------------- //

use crate::{cli::Cli, settings::bench_settings::BenchSettings, test::Test};

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.test {
        Test::Awake { settings } => awake::run(&BenchSettings::read(&settings)?, &settings),
    }
}
