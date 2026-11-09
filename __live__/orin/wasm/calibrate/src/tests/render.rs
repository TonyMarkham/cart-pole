use crate::{
    board::{answer::Answer, reading::Reading, travel::Travel},
    measure::{
        fit_report::FitReport, frame_boxes::FrameBoxes, lens_used::LensUsed,
        live_status::LiveStatus, marker_box::MarkerBox,
    },
    render::{boxes, fit, live_boxes, live_line, rail, rows},
    session::{Session, entry::Entry},
};

/// A marker's box 20 by 10 pixels, centred at (100, 50).
const MARKER: MarkerBox = MarkerBox {
    cx: 100.0,
    cy: 50.0,
    w: 20.0,
    h: 10.0,
};

/// A live view over 60 frames: Left found in every one and in the latest, End
/// in 12 and missed in the latest, the other two never found.
fn live_status() -> LiveStatus {
    LiveStatus {
        on: true,
        message: String::new(),
        per_s: 43.6,
        window: 60,
        found: [("Left".to_string(), 60), ("End".to_string(), 12)].into(),
        frame: None,
        boxes: Some(
            [
                ("Left".to_string(), Some(MARKER)),
                ("End".to_string(), None),
            ]
            .into(),
        ),
        width: 728,
        height: 544,
    }
}

/// One finished move to the motor's end, whose answer never reached the page,
/// saying `message`, at `clock`.
fn entry(clock: &str, message: &str) -> Entry {
    Entry {
        clock: clock.to_string(),
        travel: Travel::Motor,
        answer: Answer::lost(message.to_string()),
        took_s: 1.0,
    }
}

#[test]
fn given_a_new_page_when_the_rail_is_drawn_then_it_says_the_cart_is_not_known() {
    // Given
    let session = Session::new();

    // When
    let svg = rail(&session).unwrap_or_default();

    // Then
    assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
    assert!(svg.contains("Where?") && svg.contains("no reading"));
}

#[test]
fn given_a_failed_reading_when_the_rail_is_drawn_then_no_switch_has_a_state() {
    // Given
    let mut session = Session::new();
    session.read(Reading::Failed("lost".to_string()), "now".to_string());

    // When
    let svg = rail(&session).unwrap_or_default();

    // Then
    assert!(!svg.contains("pressed") && !svg.contains("released"));
}

#[test]
fn given_a_new_page_when_the_rail_is_drawn_then_the_idler_is_left_of_the_motor() {
    // Given
    let session = Session::new();

    // When
    let svg = rail(&session).unwrap_or_default();

    // Then
    assert!(svg.find(">Idler<") < svg.find(">Motor<"));
    assert!(svg.contains("x=\"50\" y=\"198\">Idler") && svg.contains("x=\"810\" y=\"198\">Motor"));
}

#[test]
fn given_a_frame_that_missed_end_when_its_boxes_are_drawn_then_only_pivot_has_one() {
    // Given
    let frame = FrameBoxes {
        pivot: Some(MARKER),
        end: None,
    };

    // When
    let drawn = boxes(&frame, 728, 544).unwrap_or_default();

    // Then
    assert!(drawn.contains("box-pivot") && !drawn.contains("box-end"));
    assert!(drawn.contains("x=\"90.0\" y=\"45.0\" width=\"20.0\" height=\"10.0\""));
}

#[test]
fn given_a_live_view_when_its_boxes_are_drawn_then_only_the_found_markers_are_named() {
    // Given
    let status = live_status();

    // When
    let svg = live_boxes(&status).unwrap_or_default();

    // Then
    assert!(svg.contains("class=\"box-left\" x=\"90.0\" y=\"45.0\""));
    assert!(svg.contains(">Left</text>") && !svg.contains(">End</text>"));
}

#[test]
fn given_a_live_view_when_its_line_is_written_then_it_counts_every_marker() {
    // Given
    let status = live_status();

    // When
    let said = live_line(&status);

    // Then
    assert_eq!(
        said,
        "Live, 44 frames a second. Found in the last 60 frames: Left 60, Right 0, Pivot 0, End 12."
    );
}

#[test]
fn given_a_fit_over_the_limit_when_reported_then_it_is_stored_and_can_be_accepted() {
    // Given
    let report = FitReport {
        within_limit: false,
        accepted: false,
        residuals: Vec::new(),
        worst_px: 8.0,
        limit_px: 1.0,
        lens: LensUsed {
            date: "2026-11-03T00:13:38Z".to_string(),
            rms_px: 0.26,
        },
        worst_uncorrected_px: 1.6,
        kept_minutes_ago: [("motor upright".to_string(), 3)].into(),
        gaps_mm: [("motor".to_string(), 5.0), ("idler".to_string(), 7.5)].into(),
        span_pulses: 59_822,
        span_m: 0.89733,
        cad_length_m: 0.873,
        difference_m: 0.02433,
        saved_to: Some("~/live-orin-python/homography.json".to_string()),
        dated_copy: Some("~/live-orin-python/saved/homography-20261103-004500.json".to_string()),
    };

    // When
    let text = fit(&report).unwrap_or_default();

    // Then
    assert!(text.contains("over the limit") && text.contains("not a failure"));
    assert!(text.contains("Stored in <code>~/live-orin-python/homography.json</code>"));
    assert!(text.contains("saved/homography-20261103-004500.json"));
    assert!(text.contains("897.3 mm") && text.contains("873.0 mm"));
    assert!(text.contains("-24.3 mm"));
    assert!(text.contains("5 mm at the motor") && text.contains("7.5 mm at the idler"));
    assert!(text.contains("2026-11-03T00:13:38Z") && text.contains("0.26 px"));
    assert!(text.contains("1.600 px"));
    assert!(text.contains("motor upright 3 min ago"));
}

#[test]
fn given_an_answer_holding_html_when_the_table_is_drawn_then_it_is_escaped() {
    // Given
    let entries = [entry("now", "<b>&\"")];

    // When
    let table = rows(&entries).unwrap_or_default();

    // Then
    assert!(table.contains("&lt;b&gt;&amp;&quot;"));
    assert!(!table.contains("<b>"));
}

#[test]
fn given_two_moves_when_the_table_is_drawn_then_the_newest_comes_first() {
    // Given
    let entries = [entry("first", "said"), entry("second", "said")];

    // When
    let table = rows(&entries).unwrap_or_default();

    // Then
    assert!(table.find("second") < table.find("first"));
}
