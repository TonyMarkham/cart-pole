//! Each run's record on the Orin (`real-loop-dia.md`, *The log*). Nothing on
//! the rig knows the true state, so unlike the synthetic record there is no
//! truth in it: what the detector saw, what the policy was given, and what was
//! sent.

pub(crate) mod civil;
pub(crate) mod frame_line;
pub(crate) mod run_log;
