//! Checks of the gaps' arithmetic and the copied contracts, each against
//! values worked out by hand.

use crate::{command::Command, stage_record::StageRecord, worker::ready_line::ReadyLine};

/// A frame that arrived at 10 ms, was in the worker's pipe at 10.5 ms, had
/// its boxes back at 23 ms, its inputs built at 23.1 ms, a back at 24.6 ms
/// and its datagram sent at 24.65 ms. The gaps are the differences of
/// neighbours: 0.5, 12.5, 0.1, 1.5 and 0.05 ms, adding up to
/// 24.65 − 10 = 14.65 ms.
#[test]
fn given_a_passes_six_moments_when_its_gaps_are_taken_then_they_are_the_hand_worked_ones() {
    // Given
    let millis = 1e6;
    let record = StageRecord {
        seq: 31,
        stamp_ns: 0,
        arrived_ns: 10.0 * millis,
        frame_sent_ns: 10.5 * millis,
        boxes_ns: 23.0 * millis,
        inputs_ns: 23.1 * millis,
        answered_ns: 24.6 * millis,
        sent_ns: 24.65 * millis,
        found: 4,
        a: 0.0,
        c: 0,
    };

    // When
    let gaps = record.gaps_ms();

    // Then
    let expected = [0.5, 12.5, 0.1, 1.5, 0.05];
    for (gap, want) in gaps.iter().zip(expected) {
        assert!((gap - want).abs() < 1e-9, "{gap} is not {want}");
    }
    assert!((record.total_ms() - 14.65).abs() < 1e-9);
    assert!((gaps.iter().sum::<f64>() - record.total_ms()).abs() < 1e-9);
}

/// The brain's command check, copied with the command: 10,000 is 0x2710,
/// sent negative as [0, 0x10, 0x27].
#[test]
fn given_a_negative_command_when_packed_then_the_sign_byte_is_0_and_the_size_little_endian() {
    assert_eq!(Command { rate: -10_000 }.to_bytes(), [0, 0x10, 0x27]);
}

/// tanh(0.5) = 0.46211715726, and 0.46211715726 × 44,600 = 20,610.4, so the
/// command is 20,610 pulses/s.
#[test]
fn given_the_policys_answer_when_made_a_command_then_it_is_a_times_the_top_rate_rounded() {
    assert_eq!(Command::from_action(0.46211715726, 44_600).rate, 20_610);
}

#[test]
fn given_the_workers_ready_line_when_parsed_then_the_policys_values_are_read() {
    // Given
    let line = r#"{"ready": true, "top_pulse_rate": 44600, "age_unit_ms": 1000.0, "past_commands": 6,
                  "command_rate_min_hz": 20.0, "command_rate_max_hz": 30.0, "source": "run 1"}"#;

    // When
    let values = ReadyLine::parse(line).unwrap();

    // Then
    assert_eq!(values.top_pulse_rate, 44_600);
    assert_eq!(values.past_commands, 6);
    assert!((values.age_unit_ms - 1_000.0).abs() < 1e-12);
    assert_eq!(values.source, "run 1");
}
