mod pole;

// ---------------------------------------------------------------------------------------------- //

use crate::measure::{parse_count, parse_gap, refusal_words};

/// The fewest and most images the window allows, as the service sends them.
const LIMITS: (u32, u32) = (30, 9_999);

/// The most gap the window allows, millimetres.
const MOST_GAP_MM: f64 = 50.0;

#[test]
fn given_whole_numbers_inside_the_limits_when_parsed_as_a_count_then_each_is_taken() {
    // Given
    let typed = [("30", 30), (" 1000 ", 1_000), ("9999", 9_999)];

    // When
    let parsed: Vec<_> = typed
        .iter()
        .map(|(text, _)| parse_count(text, LIMITS))
        .collect();

    // Then
    let expected: Vec<_> = typed.iter().map(|&(_, count)| Ok(count)).collect();
    assert_eq!(parsed, expected);
}

#[test]
fn given_anything_but_a_whole_number_in_the_limits_when_parsed_as_a_count_then_it_is_refused() {
    // Given
    let typed = [
        "29",
        "0",
        "10000",
        "",
        " ",
        "abc",
        "1e3",
        "30.5",
        "-30",
        "+30",
        "99999999999",
    ];

    // When
    let taken: Vec<&str> = typed
        .into_iter()
        .filter(|text| parse_count(text, LIMITS).is_ok())
        .collect();

    // Then
    assert!(taken.is_empty(), "{taken:?}");
}

#[test]
fn given_millimetres_inside_the_limit_when_parsed_as_a_gap_then_each_is_taken() {
    // Given
    let typed = [("5", 5.0), (" 5.25 ", 5.25), ("0", 0.0), ("50", 50.0)];

    // When
    let parsed: Vec<_> = typed
        .iter()
        .map(|(text, _)| parse_gap(text, MOST_GAP_MM))
        .collect();

    // Then
    let expected: Vec<_> = typed.iter().map(|&(_, gap)| Ok(gap)).collect();
    assert_eq!(parsed, expected);
}

#[test]
fn given_anything_but_millimetres_inside_the_limit_when_parsed_as_a_gap_then_each_is_refused() {
    // Given
    let typed = [
        "", " ", "50.1", "-5", "5mm", "5,5", "abc", "1e1", ".", "1.2.3",
    ];

    // When
    let taken: Vec<&str> = typed
        .into_iter()
        .filter(|text| parse_gap(text, MOST_GAP_MM).is_ok())
        .collect();

    // Then
    assert!(taken.is_empty(), "{taken:?}");
}

#[test]
fn given_a_refusal_when_its_words_are_read_then_they_are_its_message() {
    // Given
    let refusal = r#"{"message":"no"}"#;

    // When
    let words = refusal_words(refusal);

    // Then
    assert_eq!(words, "no");
}

#[test]
fn given_plain_text_when_its_words_are_read_then_they_are_the_whole_text() {
    // Given
    let plain = "plain";

    // When
    let words = refusal_words(plain);

    // Then
    assert_eq!(words, "plain");
}
