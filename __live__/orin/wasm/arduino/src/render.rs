use crate::{
    constants::{BOARD_X, BOARD_Y, BOTTOM_Y, TOP_Y},
    pins::{Levels, PINS},
};

use std::fmt::Write;

/// The parts of the board that never change.
const BOARD_PARTS: &str = include_str!("board_parts.svg");

/// The header holes' x positions, in the board's units: the 0.1 inch spacing of
/// the header, with the wider gap the Uno shape has in the middle of each row.
const TOP_ROW: [(&str, i32); 18] = [
    ("SCL", 245), ("SDA", 272), ("AREF", 300), ("GND", 327), ("D13", 355), ("D12", 382),
    ("D11", 409), ("D10", 437), ("D9", 464), ("D8", 491), ("D7", 536), ("D6", 563),
    ("D5", 590), ("D4", 618), ("D3", 645), ("D2", 673), ("D1", 700), ("D0", 728),
];
const BOTTOM_ROW: [(&str, i32); 14] = [
    ("BOOT", 344), ("IOREF", 372), ("RESET", 399), ("3.3V", 427), ("5V", 454), ("GND", 481),
    ("GND", 509), ("VIN", 536), ("A0", 590), ("A1", 618), ("A2", 645), ("A3", 673),
    ("A4", 700), ("A5", 727),
];

/// The whole drawing, as SVG. Everything written here comes from the constants
/// in this crate, so none of it needs escaping; only `levels` picks among them.
pub(crate) fn board(levels: Option<&Levels>) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    write!(
        out,
        "<svg viewBox=\"0 0 860 930\" role=\"img\" aria-label=\"Drawing of the Arduino UNO R4 WiFi \
         with the pins the rig uses marked. The table below gives the same information.\">"
    )?;
    for pin in &PINS {
        let class = pin.shown(levels).tone.class();
        write!(out, "<path class=\"leader {class}\" d=\"{}\"/>", pin.leader)?;
    }
    for pin in &PINS {
        let shown = pin.shown(levels);
        let (x, y, width) = pin.card;
        write!(
            out,
            "<rect class=\"card\" x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"110\" rx=\"8\"/>\
             <text class=\"card-pin\" x=\"{tx}\" y=\"{}\">{}</text>\
             <text class=\"card-role\" x=\"{tx}\" y=\"{}\">{}</text>\
             <text class=\"card-state {}\" x=\"{tx}\" y=\"{}\">{}</text>\
             <text class=\"card-level\" x=\"{tx}\" y=\"{}\">{}</text>",
            y + 27,
            pin.name,
            y + 47,
            pin.role,
            shown.tone.class(),
            y + 72,
            shown.text,
            y + 93,
            shown.level,
            tx = x + 12,
        )?;
    }
    write!(out, "<g transform=\"translate({BOARD_X} {BOARD_Y})\">{BOARD_PARTS}")?;
    for column in 0..12 {
        for row in 0..8 {
            write!(
                out,
                "<circle fill=\"var(--led)\" cx=\"{}\" cy=\"{}\" r=\"6\"/>",
                412 + 23 * column,
                300 + 23 * row
            )?;
        }
    }
    header_row(&mut out, &TOP_ROW, TOP_Y, true, levels)?;
    header_row(&mut out, &BOTTOM_ROW, BOTTOM_Y, false, levels)?;
    write!(out, "</g></svg>")?;
    Ok(out)
}

/// One header: its strip, its holes, and each hole's label, turned to read up
/// the board, below the top row's holes and above the bottom row's.
fn header_row(
    out: &mut String,
    row: &[(&str, i32)],
    y: i32,
    label_below: bool,
    levels: Option<&Levels>,
) -> std::fmt::Result {
    if let (Some((_, first)), Some((_, last))) = (row.first(), row.last()) {
        write!(
            out,
            "<rect fill=\"var(--header)\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"28\"/>",
            first - 14,
            y - 14,
            last - first + 28
        )?;
    }
    for (name, x) in row {
        let fill = PINS
            .iter()
            .find(|pin| pin.holes.contains(&(*x, y)))
            .map_or("hole", |pin| pin.shown(levels).tone.class());
        write!(
            out,
            "<rect fill=\"var(--{fill})\" x=\"{}\" y=\"{}\" width=\"14\" height=\"14\"/>",
            x - 7,
            y - 7
        )?;
        let (label_y, anchor) = if label_below { (y + 21, "end") } else { (y - 21, "start") };
        let label_x = f64::from(*x) + 4.5;
        write!(
            out,
            "<text class=\"silk\" x=\"{label_x}\" y=\"{label_y}\" text-anchor=\"{anchor}\" \
             transform=\"rotate(-90 {label_x} {label_y})\">{name}</text>"
        )?;
    }
    Ok(())
}

/// The table's rows.
pub(crate) fn rows(levels: Option<&Levels>) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    for pin in &PINS {
        let shown = pin.shown(levels);
        write!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td>\
             <td class=\"state\" style=\"color:var(--{})\">{}",
            pin.name,
            pin.chip,
            pin.wired,
            pin.does,
            shown.tone.class(),
            shown.text
        )?;
        if levels.is_some() && shown.text != "Not read" && shown.text != "No reading" {
            write!(out, ": {}", shown.level)?;
        }
        write!(out, "</td></tr>")?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resting() -> Levels {
        ["d2", "d3"]
            .map(|key| (key.to_string(), 0))
            .into_iter()
            .chain(["d4", "d5", "d6"].map(|key| (key.to_string(), 1)))
            .collect()
    }

    #[test]
    fn the_drawing_shows_every_pins_state() {
        let svg = board(Some(&resting())).unwrap_or_default();
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(svg.contains("Driver on") && svg.contains("Released"));
        assert!(!svg.contains("No reading"));
    }

    #[test]
    fn without_a_reading_no_level_survives() {
        let svg = board(None).unwrap_or_default();
        let table = rows(None).unwrap_or_default();
        for text in ["Driver on", "Released", "High (1)", "Low (0)"] {
            assert!(!svg.contains(text) && !table.contains(text), "{text}");
        }
        assert!(svg.contains("No reading") && table.contains("No reading"));
    }

    #[test]
    fn every_used_hole_is_on_a_header_under_its_own_name() {
        let on_row = |name: &str, hole: &(i32, i32)| {
            let row: &[(&str, i32)] = if hole.1 == TOP_Y { &TOP_ROW } else { &BOTTOM_ROW };
            row.contains(&(name, hole.0))
        };
        for pin in &PINS {
            assert!(pin.holes.iter().all(|hole| on_row(pin.name, hole)), "{}", pin.name);
        }
    }

    #[test]
    fn the_gnd_beside_vin_is_not_marked() {
        let marked = PINS.iter().any(|pin| pin.holes.contains(&(509, BOTTOM_Y)));
        assert!(!marked);
    }
}
