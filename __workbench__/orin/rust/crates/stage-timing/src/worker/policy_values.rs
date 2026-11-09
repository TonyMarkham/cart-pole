use anyhow::{Result, bail};

/// The values the policy trained with, from `policy.npz` through the worker's
/// ready line (the brain's `worker/policy_values.rs`): the inputs and the
/// command are built with these.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PolicyValues {
    /// Pulses/s: a = 1 asks for it.
    pub(crate) top_pulse_rate: u32,
    /// Each age is divided by this before the network sees it, ms.
    pub(crate) age_unit_ms: f64,
    /// How many past values of a the inputs end with.
    pub(crate) past_commands: usize,
    /// The run, the save and the commit it came from.
    pub(crate) source: String,
}

impl PolicyValues {
    /// Refuses values the command's three bytes can't carry: the size of c is a
    /// `u16`, so the top rate must fit one.
    pub(crate) fn check(self) -> Result<PolicyValues> {
        if self.top_pulse_rate > u32::from(u16::MAX) {
            bail!(
                "the policy's top pulse rate, {}, does not fit the command's u16",
                self.top_pulse_rate
            );
        }
        if !(self.age_unit_ms.is_finite() && self.age_unit_ms > 0.0) {
            bail!(
                "the policy's age unit must be a positive number of ms, not {}",
                self.age_unit_ms
            );
        }
        Ok(self)
    }
}
