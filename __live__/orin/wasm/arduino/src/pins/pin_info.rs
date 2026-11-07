use crate::pins::{levels::Levels, shown::Shown, source::Source, tone::Tone};

/// One pin the page draws: its card's words, where the card and its leader
/// sit, and what its level means, if the firmware reports it.
pub(crate) struct PinInfo {
    pub(crate) name: &'static str,
    pub(crate) role: &'static str,
    pub(crate) chip: &'static str,
    pub(crate) wired: &'static str,
    pub(crate) does: &'static str,
    pub(crate) source: Source,
    /// The card's top left corner and width, in the drawing's units.
    pub(crate) card: (i32, i32, i32),
    /// The path from the card's edge to the pin.
    pub(crate) leader: &'static str,
    /// The header holes it marks, in the board's units.
    pub(crate) holes: &'static [(i32, i32)],
}

impl PinInfo {
    /// Old levels are never kept: with no reading, a live pin says so, so a
    /// dead link can never pass for a live one.
    pub(crate) fn shown(&self, levels: Option<&Levels>) -> Shown {
        let Source::Live { key, high, low } = &self.source else {
            return Shown {
                text: "Not read",
                tone: Tone::None,
                level: "No live status",
            };
        };
        match levels.and_then(|levels| levels.get(*key)) {
            Some(1) => Shown {
                text: high.text,
                tone: high.tone,
                level: "High (1)",
            },
            Some(0) => Shown {
                text: low.text,
                tone: low.tone,
                level: "Low (0)",
            },
            _ => Shown {
                text: "No reading",
                tone: Tone::None,
                level: "Board not answering",
            },
        }
    }
}
