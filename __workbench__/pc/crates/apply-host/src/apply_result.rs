use crate::{
    protocol::{KIND_APPLIED, KIND_EARLY, KIND_NONE},
    result_kind::ResultKind,
};

/// One line of the R3's results: `<kind> <ticks> <old period> <new period>`,
/// every number in the R3's Timer1 counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ApplyResult {
    pub(crate) kind: ResultKind,
    /// From the mark to the first falling edge at the new rate, or, for
    /// `Early`, from that edge to the mark; 0 for `None`.
    pub(crate) ticks: u64,
    /// The steady period before the mark, and the first period after it that
    /// differed, counts; 0 for `None`'s new period.
    pub(crate) old_period: u64,
    pub(crate) new_period: u64,
}

impl ApplyResult {
    /// The result in `line`, or None for any other line.
    pub(crate) fn parse(line: &str) -> Option<Self> {
        let mut fields = line.split_whitespace();
        let kind = match fields.next()? {
            KIND_APPLIED => ResultKind::Applied,
            KIND_EARLY => ResultKind::Early,
            KIND_NONE => ResultKind::None,
            _ => return None,
        };
        let mut number = || fields.next()?.parse::<u64>().ok();
        let (ticks, old_period, new_period) = (number()?, number()?, number()?);
        Some(Self {
            kind,
            ticks,
            old_period,
            new_period,
        })
    }
}
