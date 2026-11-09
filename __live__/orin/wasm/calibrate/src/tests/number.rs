use crate::number::grouped;

#[test]
fn given_whole_numbers_when_grouped_then_their_digits_are_split_in_threes_by_commas() {
    // Given
    let numbers = [
        (0, "0"),
        (999, "999"),
        (1_000, "1,000"),
        (59_821, "59,821"),
        (1_234_567, "1,234,567"),
    ];

    // When
    let written: Vec<String> = numbers.iter().map(|&(number, _)| grouped(number)).collect();

    // Then
    let expected: Vec<&str> = numbers.iter().map(|&(_, text)| text).collect();
    assert_eq!(written, expected);
}
