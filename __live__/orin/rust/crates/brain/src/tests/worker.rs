use crate::worker::{detection::Detection, policy_values::PolicyValues, ready_line::ReadyLine};

#[test]
fn given_the_workers_ready_line_when_parsed_then_the_policys_values_are_read() {
    // Given
    let line = r#"{"ready": true, "top_pulse_rate": 44600, "age_unit_ms": 1000.0, "past_commands": 6,
                  "source": "run 20261108_115227"}"#;

    // When
    let values = ReadyLine::parse(line).unwrap();

    // Then
    assert_eq!(
        values,
        PolicyValues {
            top_pulse_rate: 44_600,
            age_unit_ms: 1_000.0,
            past_commands: 6,
            source: "run 20261108_115227".to_string(),
        }
    );
}

#[test]
fn given_the_workers_error_line_when_parsed_then_its_reason_is_the_error() {
    // Given
    let line = r#"{"error": "no policy file"}"#;

    // When
    let parsed = ReadyLine::parse(line);

    // Then
    assert!(
        parsed
            .unwrap_err()
            .to_string()
            .contains("the worker could not start: no policy file")
    );
}

#[test]
fn given_a_line_that_is_not_json_when_parsed_then_it_is_refused() {
    assert!(ReadyLine::parse("detector loaded").is_err());
}

#[test]
fn given_a_top_rate_past_the_u16_when_checked_then_the_values_are_refused() {
    // Given
    let values = PolicyValues {
        top_pulse_rate: 70_000,
        age_unit_ms: 1_000.0,
        past_commands: 6,
        source: String::new(),
    };

    // When
    let checked = values.check();

    // Then
    assert!(
        checked
            .unwrap_err()
            .to_string()
            .contains("does not fit the command's u16")
    );
}

#[test]
fn given_the_workers_twenty_bytes_when_read_then_they_are_a_detection() {
    // Given
    let mut bytes = [0u8; 20];
    for (chunk, value) in bytes
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip([0.5f32, 0.25, 0.1, 0.05, 0.9])
    {
        *chunk = value.to_le_bytes();
    }

    // When
    let detection = Detection::from_bytes(&bytes);

    // Then
    assert!(detection.found());
    assert_eq!(
        detection.as_array(),
        [
            0.5,
            0.25,
            f64::from(0.1f32),
            f64::from(0.05f32),
            f64::from(0.9f32)
        ]
    );
}

#[test]
fn given_a_marker_not_found_when_read_then_it_is_not_found() {
    // Given
    let mut bytes = [0u8; 20];
    bytes[..4].copy_from_slice(&(-1.0f32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(-1.0f32).to_le_bytes());

    // When
    let detection = Detection::from_bytes(&bytes);

    // Then
    assert!(!detection.found());
}
