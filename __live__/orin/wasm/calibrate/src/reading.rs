use crate::switches::Switches;

/// What one ask for the switches found.
#[derive(Debug, PartialEq)]
pub(crate) enum Reading {
    Levels(Switches),
    /// The webserver says a move is under way, one this page did not start: from
    /// another open page, or from this one before it was refreshed.
    MovingElsewhere,
    Failed(String),
}
