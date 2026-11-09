use crate::pins::tone::Tone;

/// What a level means on this rig.
#[derive(Clone, Copy)]
pub(crate) struct Meaning {
    pub(crate) text: &'static str,
    pub(crate) tone: Tone,
}

impl Meaning {
    pub(crate) const fn new(text: &'static str, tone: Tone) -> Self {
        Self { text, tone }
    }
}
