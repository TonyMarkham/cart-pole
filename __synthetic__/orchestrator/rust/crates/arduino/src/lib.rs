//! The cart's Arduino firmware (synthetic-arduino-plan.md): one source for the
//! rig and the sim loop. The Orin sends a target step-pulse rate (4
//! bytes, `i32` little-endian, pulses/s, sign = direction); the firmware
//! clamps it to the top pulse rate, ramps the sent rate towards it at no
//! more than the jump limit m (`Ramp`, the acceleration ceiling) and sets
//! the DM542's two signals, DIR from the sign and STEP rate from the
//! magnitude of the sent rate. On the board, the step generator emits
//! whole pulses at that rate, never past the rail's travel envelope.
//!
//! The core is `no_std` and integer-only, the shape the board's build
//! needs. The `synthetic` feature adds the sim loop's edge,
//! `SyntheticArduino`: commands come in by call instead of radio, and
//! the DM542 signals go back to the orchestrator, which turns them into
//! the MuJoCo actuator's `ctrl`; the loop's clock is MuJoCo's timestep, which
//! ticks the ramp and the limit switches but counts no pulses. The board's
//! own edge (radio, step pin, timer) is a later `live` feature.

#![cfg_attr(not(test), no_std)]

mod command;
mod config;
mod direction;
#[cfg(feature = "synthetic")]
mod limit_switches;
mod ramp;
mod signals;
mod stepper;
#[cfg(feature = "synthetic")]
mod synthetic;
#[cfg(test)]
mod tests;
#[cfg(feature = "synthetic")]
mod tick_answer;

// ---------------------------------------------------------------------------------------------- //

pub use crate::{
    command::{COMMAND_LEN, decode_command, encode_command},
    config::Config,
    direction::Direction,
    ramp::Ramp,
    signals::Dm542Signals,
    stepper::Stepper,
};
#[cfg(feature = "synthetic")]
pub use crate::{
    limit_switches::LimitSwitches, synthetic::SyntheticArduino, tick_answer::TickAnswer,
};
