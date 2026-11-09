//! Gaps 2 to 6 of `real-timings.md` on the Orin: how long each stage of the
//! brain's loop takes, from a frame arriving to its command leaving.
//!
//! The brain's log stamps only the two ends, `arrived` and `sent`. This runs
//! the brain's loop as the brain does it, with the same camera, the same
//! unchanged Python worker, the same matrix and the same policy, and stamps
//! every stage's end on one clock, the Orin's monotonic clock
//! (`stage_run.rs`). One run gives all five gaps for the same frames, so for
//! every frame they add up to `sent` less `arrived`.
//!
//! The one change from the brain: the datagrams go to `[send]`'s address,
//! not the rig's Arduino, so the cart never moves.
//!
//! `stage-timing stage-timing.toml`, on the Orin, with no run going in the
//! brain and `calibrate_service.py` not using the camera.

mod camera;
mod cli;
mod command;
mod constants;
mod geometry;
mod history;
mod home;
mod inputs;
mod pool;
mod report;
mod run_folder;
mod sender;
mod settings;
mod stage_record;
mod stage_run;
mod stats;
#[cfg(test)]
mod tests;
mod worker;

// ---------------------------------------------------------------------------------------------- //

use crate::{cli::Cli, settings::stage_settings::StageSettings};

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    stage_run::run(&StageSettings::read(&cli.settings)?, &cli.settings)
}
