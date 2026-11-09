use crate::trainer_link::next::Next;

use serde::Deserialize;

/// 18: one instance's answer: `{"instance": k, "command": c, "next":
/// "continue" | "reset" | "stop"}`, plus `"seed": S` on a reset only.
/// serde rejects a command that is not an integer or does not fit an i32,
/// and a seed outside a u64, so a bad answer fails the run here instead of
/// reaching the synthetic Arduino.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub instance: usize,
    pub command: i32,
    pub next: Next,
    pub seed: Option<u64>,
}
