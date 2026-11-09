use crate::{
    camera::{gdp_header::GdpHeader, latest::Latest, read_frames},
    constants::{FRAME_BYTES, GDP_HEADER_LENGTH},
};

use std::{io::Cursor, time::Duration};

/// A GDP header as gdppay writes it: version 1.0, the type at byte 4, the
/// length at byte 6 and the timestamp at byte 10, all big-endian.
fn header(payload_type: u16, length: u32, timestamp_ns: u64) -> [u8; GDP_HEADER_LENGTH] {
    let mut bytes = [0u8; GDP_HEADER_LENGTH];
    bytes[0] = 1;
    bytes[4..6].copy_from_slice(&payload_type.to_be_bytes());
    bytes[6..10].copy_from_slice(&length.to_be_bytes());
    bytes[10..18].copy_from_slice(&timestamp_ns.to_be_bytes());
    bytes
}

#[test]
fn given_a_buffer_header_when_parsed_then_its_three_fields_are_read() {
    // Given
    let bytes = header(1, FRAME_BYTES as u32, 1_234_567_890);

    // When
    let parsed = GdpHeader::parse(&bytes).unwrap();

    // Then
    assert!(parsed.is_buffer());
    assert_eq!(parsed.payload_length, FRAME_BYTES as u32);
    assert_eq!(parsed.timestamp_ns, 1_234_567_890);
}

#[test]
fn given_a_header_of_another_version_when_parsed_then_it_is_refused() {
    // Given
    let mut bytes = header(1, 10, 0);
    bytes[0] = 0;

    // When
    let parsed = GdpHeader::parse(&bytes);

    // Then
    assert!(parsed.unwrap_err().contains("not GDP version 1"));
}

#[test]
fn given_a_caps_packet_then_a_frame_when_read_then_the_frame_is_the_latest_with_its_stamp() {
    // Given
    let mut pipe = Vec::new();
    pipe.extend_from_slice(&header(2, 5, u64::MAX));
    pipe.extend_from_slice(b"caps\0");
    pipe.extend_from_slice(&header(1, FRAME_BYTES as u32, 16_700_000));
    pipe.extend(std::iter::repeat_n(7u8, FRAME_BYTES));
    let latest = Latest::default();

    // When
    let ended = read_frames(Cursor::new(pipe), &latest);

    // Then
    assert!(ended.is_ok());
    let frame = latest.next_after(0, Duration::from_millis(10)).unwrap();
    assert_eq!(frame.seq, 1);
    assert_eq!(frame.stamp_ns, 16_700_000);
    assert_eq!(frame.pixels.len(), FRAME_BYTES);
    assert!(frame.pixels.iter().all(|&pixel| pixel == 7));
}

#[test]
fn given_a_frame_of_the_wrong_size_when_read_then_the_reader_stops_saying_so() {
    // Given
    let mut pipe = Vec::new();
    pipe.extend_from_slice(&header(1, 100, 0));
    pipe.extend_from_slice(&[0u8; 100]);
    let latest = Latest::default();

    // When
    let ended = read_frames(Cursor::new(pipe), &latest);

    // Then
    assert!(ended.unwrap_err().contains("a frame of 100 bytes"));
}

#[test]
fn given_the_latest_frame_when_the_next_after_it_is_asked_for_then_the_wait_runs_out() {
    // Given
    let latest = Latest::default();
    let mut pipe = Vec::new();
    pipe.extend_from_slice(&header(1, FRAME_BYTES as u32, 1));
    pipe.extend(std::iter::repeat_n(0u8, FRAME_BYTES));
    read_frames(Cursor::new(pipe), &latest).unwrap();

    // When
    let next = latest.next_after(1, Duration::from_millis(20));

    // Then
    assert!(
        next.unwrap_err()
            .to_string()
            .contains("no frame within 20 ms")
    );
}
