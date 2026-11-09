use crate::command::Command;

#[test]
fn given_the_policys_answer_when_made_a_command_then_it_is_a_times_the_top_rate_rounded() {
    // Given: tanh(0.5) = 0.46211715726, and 0.46211715726 x 44,600 = 20,610.4
    let a = 0.46211715726;

    // When
    let command = Command::from_action(a, 44_600);

    // Then
    assert_eq!(command.rate, 20_610);
}

#[test]
fn given_an_answer_past_the_band_when_made_a_command_then_it_is_clipped_to_the_top_rate() {
    assert_eq!(Command::from_action(1.5, 44_600).rate, 44_600);
    assert_eq!(Command::from_action(-1.0, 44_600).rate, -44_600);
}

#[test]
fn given_a_negative_command_when_packed_then_the_sign_byte_is_0_and_the_size_little_endian() {
    // Given: 10,000 is 0x2710
    let command = Command { rate: -10_000 };

    // When
    let bytes = command.to_bytes();

    // Then
    assert_eq!(bytes, [0, 0x10, 0x27]);
}

#[test]
fn given_the_top_rate_when_packed_then_the_sign_byte_is_1_and_the_size_fits_the_u16() {
    // Given: 44,600 is 0xAE38
    let command = Command { rate: 44_600 };

    // When
    let bytes = command.to_bytes();

    // Then
    assert_eq!(bytes, [1, 0x38, 0xAE]);
}

#[test]
fn given_rest_when_packed_then_it_is_a_positive_zero() {
    assert_eq!(Command { rate: 0 }.to_bytes(), [1, 0, 0]);
}
