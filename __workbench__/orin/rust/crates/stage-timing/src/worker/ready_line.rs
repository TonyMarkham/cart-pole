use crate::worker::policy_values::PolicyValues;

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

/// The one JSON line the worker writes before anything else: the policy's
/// values when it is ready, or the reason it is not (the brain's
/// `worker/ready_line.rs`). The line also carries the trained command rates,
/// which the brain only shows; they are left unread here.
#[derive(Debug, Deserialize)]
pub(crate) struct ReadyLine {
    #[serde(default)]
    ready: bool,
    error: Option<String>,
    top_pulse_rate: Option<u32>,
    age_unit_ms: Option<f64>,
    past_commands: Option<usize>,
    source: Option<String>,
}

impl ReadyLine {
    /// The line read, or the worker's reason for not being ready.
    pub(crate) fn parse(line: &str) -> Result<PolicyValues> {
        let parsed: ReadyLine = serde_json::from_str(line.trim())
            .with_context(|| format!("the worker's first line is not its ready line: {line:?}"))?;
        if let Some(reason) = parsed.error {
            bail!("the worker could not start: {reason}");
        }
        if !parsed.ready {
            bail!("the worker's first line says it is not ready: {line:?}");
        }
        let missing = |what: &str| anyhow!("the worker's ready line has no {what}");
        PolicyValues {
            top_pulse_rate: parsed
                .top_pulse_rate
                .ok_or_else(|| missing("top_pulse_rate"))?,
            age_unit_ms: parsed.age_unit_ms.ok_or_else(|| missing("age_unit_ms"))?,
            past_commands: parsed
                .past_commands
                .ok_or_else(|| missing("past_commands"))?,
            source: parsed.source.unwrap_or_default(),
        }
        .check()
    }
}
