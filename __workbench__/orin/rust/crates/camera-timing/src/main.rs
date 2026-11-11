//! Gap 1 of `__documentation__/real-timings.md` on the Orin: from the camera exposing a frame to
//! the frame being read, the moment the brain's log calls `arrived`.
//!
//! The frame's own stamp can't give it: GStreamer stamps a frame when it
//! passes it on, not when the sensor exposed it (`__documentation__/real-loop-dia.md`,
//! *Difference 1*). So the Orin flashes an LED in the camera's view, wired to
//! one of its GPIO pins (`__workbench__/orin/led-circuit.svg`), and stamps
//! the flash and each frame's arrival on one clock, its own monotonic clock.
//! A frame that shows the flash was exposing during it, which pins its
//! exposure to the flash's moment (`gap_one.rs`).
//!
//! `camera-timing camera-timing.toml`, on the Orin, with the brain and
//! `calibrate_service.py` not using the camera.

mod calibration;
mod camera;
mod cli;
mod constants;
mod flash;
mod flash_run;
mod frame_record;
mod gap_one;
mod led;
mod patch;
mod random;
mod report;
mod run_folder;
mod settings;
mod stats;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use crate::{cli::Cli, settings::timing_settings::TimingSettings};

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    flash_run::run(&TimingSettings::read(&cli.settings)?, &cli.settings)
}
