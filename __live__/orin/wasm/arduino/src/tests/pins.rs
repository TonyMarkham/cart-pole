use crate::pins::{PINS, levels::Levels, pin_info::PinInfo, tone::Tone};

/// A reading of D2 alone, at `d2`.
fn levels(d2: u8) -> Levels {
    Levels::from([("d2".to_string(), d2)])
}

/// The pin the page draws under `name`.
fn pin(name: &str) -> Option<&'static PinInfo> {
    PINS.iter().find(|pin| pin.name == name)
}

#[test]
fn given_d2_reads_low_when_shown_then_its_switch_is_released() {
    // Given
    let reading = levels(0);

    // When
    let shown = pin("D2").map(|pin| pin.shown(Some(&reading)).text);

    // Then
    assert_eq!(shown, Some("Released"));
}

#[test]
fn given_d2_reads_high_when_shown_then_its_switch_is_pressed_or_its_wire_open() {
    // Given
    let reading = levels(1);

    // When
    let shown = pin("D2").map(|pin| pin.shown(Some(&reading)).text);

    // Then
    assert_eq!(shown, Some("Pressed, or wire open"));
}

#[test]
fn given_no_reading_when_a_live_pin_is_shown_then_it_shows_no_level() {
    // Given
    let reading = None;

    // When
    let shown = pin("D2").map(|pin| pin.shown(reading));

    // Then
    assert_eq!(
        shown.map(|shown| (shown.text, shown.tone)),
        Some(("No reading", Tone::None))
    );
}

#[test]
fn given_a_level_neither_0_nor_1_when_shown_then_it_is_no_reading() {
    // Given
    let reading = levels(7);

    // When
    let shown = pin("D2").map(|pin| pin.shown(Some(&reading)).text);

    // Then
    assert_eq!(shown, Some("No reading"));
}

#[test]
fn given_a_reading_when_a_power_pin_is_shown_then_it_is_not_read() {
    // Given
    let reading = levels(1);

    // When
    let shown = pin("5V").map(|pin| pin.shown(Some(&reading)).text);

    // Then
    assert_eq!(shown, Some("Not read"));
}
