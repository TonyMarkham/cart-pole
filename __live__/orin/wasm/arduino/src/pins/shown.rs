use crate::pins::tone::Tone;

/// What the page shows for one pin now.
#[derive(Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) text: &'static str,
    pub(crate) tone: Tone,
    pub(crate) level: &'static str,
}
