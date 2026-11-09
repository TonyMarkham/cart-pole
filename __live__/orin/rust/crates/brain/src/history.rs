use std::collections::VecDeque;

/// The actions a of the last commands, newest first, 0 before the run's first
/// (`real-loop-dia.md`, *The pool on the rig*, the 36 inputs). The brain
/// sends the commands, so it has them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct History {
    actions: VecDeque<f64>,
}

impl History {
    /// `length` zeros: the history before the run's first command.
    pub(crate) fn new(length: usize) -> History {
        History {
            actions: std::iter::repeat_n(0.0, length).collect(),
        }
    }

    /// The command just sent becomes the newest; the oldest drops.
    pub(crate) fn push(&mut self, a: f64) {
        if self.actions.is_empty() {
            return;
        }
        self.actions.push_front(a);
        self.actions.pop_back();
    }

    /// Newest first.
    pub(crate) fn actions(&self) -> impl Iterator<Item = f64> + '_ {
        self.actions.iter().copied()
    }
}
