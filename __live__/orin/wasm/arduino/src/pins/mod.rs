pub(crate) mod levels;
pub(crate) mod meaning;
pub(crate) mod pin_info;
pub(crate) mod shown;
pub(crate) mod source;
pub(crate) mod tone;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{BOTTOM_CARD_Y, BOTTOM_Y, LIVE_CARD_WIDTH, POWER_CARD_WIDTH, TOP_CARD_Y, TOP_Y},
    pins::{meaning::Meaning, pin_info::PinInfo, source::Source, tone::Tone},
};

/// What a limit switch's level means: its contact is closed until pressed, so a
/// pressed switch and a broken wire both read high.
const PRESSED: Meaning = Meaning::new("Pressed, or wire open", Tone::Warn);
const RELEASED: Meaning = Meaning::new("Released", Tone::Ok);

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
            high: Meaning::new("Driver on", Tone::Ok),
            low: Meaning::new("Driver off, motor free", Tone::Warn),
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
            high: Meaning::new("Plus", Tone::Info),
            low: Meaning::new("Minus", Tone::Info),
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
            high: Meaning::new("Resting, no pulse", Tone::Ok),
            low: Meaning::new("In a pulse", Tone::Info),
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
