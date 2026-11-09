use crate::{constants::ROW, history::History, pool::Pool};

/// The policy's inputs, as it trained on them (`real-loop-dia.md`, *The pool
/// on the rig*): the pool's 30 numbers, newest entry first, each age divided by
/// the age unit so it reaches the network in seconds, then the actions of the
/// last commands, newest first. 36 with 6 past commands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Inputs {
    values: Vec<f64>,
}

impl Inputs {
    pub(crate) fn build(pool: &Pool, history: &History, age_unit_ms: f64) -> Inputs {
        let mut values = pool.flat().to_vec();
        for age in values.iter_mut().step_by(ROW) {
            *age /= age_unit_ms;
        }
        values.extend(history.actions());
        Inputs { values }
    }

    pub(crate) fn values(&self) -> &[f64] {
        &self.values
    }
}
