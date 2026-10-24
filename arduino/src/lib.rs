//! The cart's Arduino firmware (synthetic-arduino-plan.md): one source for the
//! rig and the sim loop. The Orin sends a target step-pulse rate (4
//! bytes, `i32` little-endian, pulses/s, sign = direction); the firmware
//! clamps it to the motor's top rate and sets the DRV8825's two signals,
//! DIR from the sign and STEP rate from the magnitude. On the board, the
//! step generator emits whole pulses at that rate, never past the rail's
//! travel envelope.
//!
//! The core is `no_std` and integer-only, the shape the board's build
//! needs. The `synthetic` feature adds the sim loop's edge,
//! `SyntheticArduino`: commands come in by call instead of radio, and
//! the DRV8825 signals go back to the orchestrator, which turns them into
//! the MuJoCo actuator's `ctrl`; the loop has no clock, so no pulses are
//! counted there. The board's own edge (radio, step pin,
//! timer) is a later `live` feature.

#![cfg_attr(not(test), no_std)]

mod command;
mod config;
mod signals;
mod stepper;
#[cfg(feature = "synthetic")]
mod synthetic;

pub use crate::{
    command::{COMMAND_LEN, decode_command, encode_command},
    config::{Config, RIG},
    signals::{Direction, Drv8825Signals},
    stepper::Stepper,
};
#[cfg(feature = "synthetic")]
pub use crate::synthetic::SyntheticArduino;

#[cfg(test)]
mod tests;
