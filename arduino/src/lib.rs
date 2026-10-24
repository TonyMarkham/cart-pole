//! The cart's Arduino firmware (synthetic-arduino-plan.md): one source for the
//! rig and the sim loop. The Orin sends a target step-pulse rate (4
//! bytes, `i32` little-endian, pulses/s, sign = direction); the firmware
//! clamps it to the motor's top rate and emits whole step pulses at that
//! rate, never past the rail's travel envelope.
//!
//! The core is `no_std` and integer-only, the shape the board's build
//! needs. The `synthetic` feature adds the sim loop's edge,
//! `SyntheticArduino`: commands come in by call instead of radio, and
//! the step signal goes back to the orchestrator, which turns it into
//! the MuJoCo actuator's `ctrl`. The board's own edge (radio, step pin,
//! timer) is a later `live` feature.

#![cfg_attr(not(test), no_std)]

mod command;
mod config;
mod stepper;
#[cfg(feature = "synthetic")]
mod synthetic;

pub use crate::{
    command::{COMMAND_LEN, decode_command, encode_command},
    config::{Config, RIG},
    stepper::Stepper,
};
#[cfg(feature = "synthetic")]
pub use crate::synthetic::{StepSignal, SyntheticArduino};

#[cfg(test)]
mod tests;
