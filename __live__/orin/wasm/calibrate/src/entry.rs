use crate::{answer::Answer, travel::Travel};

/// One finished move, for the table.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) clock: String,
    pub(crate) travel: Travel,
    pub(crate) answer: Answer,
    pub(crate) took_s: f64,
}
