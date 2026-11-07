use crate::{
    constants::{BOTTOM_Y, TOP_Y},
    pins::{PINS, levels::Levels},
    render::{BOTTOM_ROW, TOP_ROW, board, rows},
};

/// The levels of a board at rest: both switches released, the driver on, no
/// pulse, DIR high.
fn resting() -> Levels {
    ["d2", "d3"]
        .map(|key| (key.to_string(), 0))
        .into_iter()
        .chain(["d4", "d5", "d6"].map(|key| (key.to_string(), 1)))
        .collect()
}

#[test]
fn given_a_board_at_rest_when_drawn_then_every_pins_state_is_shown() {
    // Given
    let reading = resting();

    // When
    let svg = board(Some(&reading)).unwrap_or_default();

    // Then
    assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
    assert!(svg.contains("Driver on") && svg.contains("Released"));
    assert!(!svg.contains("No reading"));
}

#[test]
fn given_no_reading_when_drawn_then_no_level_survives() {
    // Given
    let reading = None;

    // When
    let svg = board(reading).unwrap_or_default();
    let table = rows(reading).unwrap_or_default();

    // Then
    for text in ["Driver on", "Released", "High (1)", "Low (0)"] {
        assert!(!svg.contains(text) && !table.contains(text), "{text}");
    }
    assert!(svg.contains("No reading") && table.contains("No reading"));
}

#[test]
fn given_the_rigs_pins_when_their_holes_are_placed_then_each_is_under_its_own_name() {
    // Given
    let pins = &PINS;

    // When
    let misplaced: Vec<&str> = pins
        .iter()
        .filter(|pin| {
            !pin.holes.iter().all(|hole| {
                let row: &[(&str, i32)] = if hole.1 == TOP_Y {
                    &TOP_ROW
                } else {
                    &BOTTOM_ROW
                };
                row.contains(&(pin.name, hole.0))
            })
        })
        .map(|pin| pin.name)
        .collect();

    // Then
    assert!(misplaced.is_empty(), "{misplaced:?}");
}

#[test]
fn given_the_rigs_pins_when_the_gnd_beside_vin_is_looked_for_then_it_is_not_marked() {
    // Given
    let gnd_beside_vin = (509, BOTTOM_Y);

    // When
    let marked = PINS.iter().any(|pin| pin.holes.contains(&gnd_beside_vin));

    // Then
    assert!(!marked);
}
