use crate::{
    live_status::{LiveStatus, marker_box::MarkerBox},
    render::{boxes, rows},
};

/// A view that is on, over 60 frames: Left found in every one, at `left`, End
/// in 12 and missed in the latest, and the other two never found.
fn status(left: Option<MarkerBox>) -> LiveStatus {
    LiveStatus {
        on: true,
        message: String::new(),
        frames: 10,
        per_s: 40.0,
        window: 60,
        found: [("Left".to_string(), 60), ("End".to_string(), 12)].into(),
        frame: None,
        boxes: Some([("Left".to_string(), left), ("End".to_string(), None)].into()),
        width: 728,
        height: 544,
    }
}

#[test]
fn given_a_found_marker_when_the_boxes_are_drawn_then_it_gets_a_named_box() {
    // Given
    let marker = MarkerBox {
        cx: 100.0,
        cy: 50.0,
        w: 20.0,
        h: 10.0,
    };

    // When
    let svg = boxes(&status(Some(marker))).unwrap_or_default();

    // Then
    assert!(
        svg.contains("class=\"box-left\" x=\"90.0\" y=\"45.0\" width=\"20.0\" height=\"10.0\"")
    );
    assert!(svg.contains(">Left</text>") && !svg.contains(">End</text>"));
}

#[test]
fn given_markers_found_in_some_frames_when_the_table_is_drawn_then_only_every_frame_reads_good() {
    // Given
    let view = status(None);

    // When
    let table = rows(&view).unwrap_or_default();

    // Then
    assert!(table.contains("class=\"good\">60 of 60"));
    assert!(table.contains("class=\"bad\">12 of 60"));
    assert!(table.contains("class=\"bad\">0 of 60"));
    assert!(table.contains("not in this frame"));
}
