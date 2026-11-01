use crate::constants::{BOTTOM_Y, TOP_Y};

use std::collections::BTreeMap;

/// One reading, as the Arduino sends it: `{"d2":0,"d3":0,...}`, 1 = high.
pub(crate) type Levels = BTreeMap<String, u8>;

/// How a state is coloured. The words carry the meaning, so nothing depends on
/// seeing the colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Tone {
    Ok,
    Warn,
    Info,
    None,
}

impl Tone {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Tone::Ok => "ok",
            Tone::Warn => "warn",
            Tone::Info => "info",
            Tone::None => "none",
        }
    }
}

/// What a level means on this rig.
#[derive(Clone, Copy)]
pub(crate) struct Meaning {
    text: &'static str,
    tone: Tone,
}

/// Whether the firmware reports a pin. `key` names it in the Arduino's answer.
pub(crate) enum Source {
    Live {
        key: &'static str,
        high: Meaning,
        low: Meaning,
    },
    NotRead,
}

/// What the page shows for one pin now.
#[derive(Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) text: &'static str,
    pub(crate) tone: Tone,
    pub(crate) level: &'static str,
}

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

const fn meaning(text: &'static str, tone: Tone) -> Meaning {
    Meaning { text, tone }
}

const PRESSED: Meaning = meaning("Pressed, or wire open", Tone::Warn);
const RELEASED: Meaning = meaning("Released", Tone::Ok);
const LIVE_CARD_WIDTH: i32 = 155;
const POWER_CARD_WIDTH: i32 = 190;
const TOP_CARD_Y: i32 = 10;
const BOTTOM_CARD_Y: i32 = 810;

/// Every pin the rig uses: `control-design.md`, *The pin map*. Chip pins are
/// unor4-hal's `gpio/pins.rs`.
pub(crate) const PINS: [PinInfo; 8] = [
    PinInfo {
        name: "D6",
        role: "ENA, motor on or off",
        chip: "P111",
        wired: "DM542 ENA+, through transistor Q3",
        does: "High leaves the driver on. Low switches it off and frees the motor.",
        source: Source::Live {
            key: "d6",
            high: meaning("Driver on", Tone::Ok),
            low: meaning("Driver off, motor free", Tone::Warn),
        },
        card: (25, TOP_CARD_Y, LIVE_CARD_WIDTH),
        leader: "M102 120V175H593V222",
        holes: &[(563, TOP_Y)],
    },
    PinInfo {
        name: "D5",
        role: "DIR, direction",
        chip: "P107",
        wired: "DM542 DIR+, through transistor Q2",
        does: "The direction of the next pulses: high is Plus, low is Minus.",
        source: Source::Live {
            key: "d5",
            high: meaning("Plus", Tone::Info),
            low: meaning("Minus", Tone::Info),
        },
        card: (190, TOP_CARD_Y, LIVE_CARD_WIDTH),
        leader: "M267 120V160H620V222",
        holes: &[(590, TOP_Y)],
    },
    PinInfo {
        name: "D4",
        role: "PUL, motor steps",
        chip: "P106",
        wired: "DM542 PUL+, through transistor Q1",
        does: "Rests high. Each low pulse moves the motor one step. This firmware sends none.",
        source: Source::Live {
            key: "d4",
            high: meaning("Resting, no pulse", Tone::Ok),
            low: meaning("In a pulse", Tone::Info),
        },
        card: (355, TOP_CARD_Y, LIVE_CARD_WIDTH),
        leader: "M432 120V145H648V222",
        holes: &[(618, TOP_Y)],
    },
    PinInfo {
        name: "D3",
        role: "Limit switch",
        chip: "P105",
        wired: "A switch's NC contact; the other contact to GND",
        does: "The rail's right end on the pin map. Low is released; high is pressed, or the wire is open.",
        source: Source::Live {
            key: "d3",
            high: PRESSED,
            low: RELEASED,
        },
        card: (520, TOP_CARD_Y, LIVE_CARD_WIDTH),
        leader: "M597 120V130H675V222",
        holes: &[(645, TOP_Y)],
    },
    PinInfo {
        name: "D2",
        role: "Limit switch",
        chip: "P104",
        wired: "A switch's NC contact; the other contact to GND",
        does: "The rail's left end on the pin map. Low is released; high is pressed, or the wire is open.",
        source: Source::Live {
            key: "d2",
            high: PRESSED,
            low: RELEASED,
        },
        card: (685, TOP_CARD_Y, LIVE_CARD_WIDTH),
        leader: "M762 120V130H703V222",
        holes: &[(673, TOP_Y)],
    },
    PinInfo {
        name: "5V",
        role: "The board's 5 V supply",
        chip: "Not a signal",
        wired: "Q1, Q2 and Q3's emitters and pull-up resistors",
        does: "Powers the three transistors that drive the DM542's inputs.",
        source: Source::NotRead,
        card: (225, BOTTOM_CARD_Y, POWER_CARD_WIDTH),
        leader: "M484 760V795H320V810",
        holes: &[(454, BOTTOM_Y)],
    },
    PinInfo {
        name: "GND",
        role: "The return path",
        chip: "Not a signal",
        wired: "PUL−, DIR−, ENA−, both switches",
        does: "The zero volts the 5 V side is measured from. It is kept apart from the 24 V supply's −.",
        source: Source::NotRead,
        card: (430, BOTTOM_CARD_Y, POWER_CARD_WIDTH),
        leader: "M511 760V785H525V810",
        holes: &[(481, BOTTOM_Y)],
    },
    PinInfo {
        name: "Power jack",
        role: "The board's power",
        chip: "Not a pin",
        wired: "The board's power supply, by the barrel jack",
        does: "Powers the board. VIN and the GND beside it are not connected.",
        source: Source::NotRead,
        card: (20, BOTTOM_CARD_Y, POWER_CARD_WIDTH),
        leader: "M115 740V810",
        holes: &[],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(d2: u8) -> Levels {
        Levels::from([("d2".to_string(), d2)])
    }

    fn pin(name: &str) -> Option<&'static PinInfo> {
        PINS.iter().find(|pin| pin.name == name)
    }

    #[test]
    fn a_switch_reads_as_the_wiring_says() {
        let d2 = pin("D2");
        assert_eq!(d2.map(|pin| pin.shown(Some(&levels(0))).text), Some("Released"));
        assert_eq!(d2.map(|pin| pin.shown(Some(&levels(1))).text), Some("Pressed, or wire open"));
    }

    #[test]
    fn no_reading_never_shows_a_level() {
        let shown = pin("D2").map(|pin| pin.shown(None));
        assert_eq!(shown.map(|shown| (shown.text, shown.tone)), Some(("No reading", Tone::None)));
    }

    #[test]
    fn a_level_outside_0_and_1_is_no_reading() {
        let shown = pin("D2").map(|pin| pin.shown(Some(&levels(7))).text);
        assert_eq!(shown, Some("No reading"));
    }

    #[test]
    fn a_power_pin_is_not_read_even_with_a_reading() {
        let shown = pin("5V").map(|pin| pin.shown(Some(&levels(1))).text);
        assert_eq!(shown, Some("Not read"));
    }
}
