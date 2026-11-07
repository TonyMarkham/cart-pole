use crate::{
    lens_status::{LensStatus, lens_rect::LensRect},
    render::{hold, line, overlay, progress, turn},
};

/// A lens view at the first of 45 positions (25 flat), its box tilted to
/// `tilt` (empty when flat): 13 of 15 corners found, `in_box` and, when
/// `turned`, ready; 3 of 10 frames held and 2 stills kept.
fn status(tilt: &str, in_box: bool, turned: bool) -> LensStatus {
    LensStatus {
        on: true,
        message: String::new(),
        per_s: 40.0,
        frame: None,
        markers: vec![vec![[10.0, 20.0], [30.0, 20.0], [30.0, 40.0], [10.0, 40.0]]],
        outline: Some(vec![[5.0, 5.0], [300.0, 5.0], [300.0, 200.0], [5.0, 200.0]]),
        corners: 13,
        all_corners: 15,
        min_corners: 11,
        in_box,
        turn_deg: None,
        ready: in_box && turned,
        width: 1456,
        height: 1088,
        distance_mm: 1000.0,
        tilt_deg: 30.0,
        min_tilt_deg: 20.0,
        rect: Some(LensRect {
            x: 0.0,
            y: 0.0,
            w: 355.0,
            h: 237.0,
            tilt: tilt.to_owned(),
            shape: vec![[29.5, 19.8], [325.5, 19.8], [325.5, 217.2], [29.5, 217.2]],
        }),
        position: 0,
        positions: 45,
        flat_positions: 25,
        held: 3,
        hold_frames: 10,
        kept: 2,
        min_views: 10,
        result: None,
    }
}

#[test]
fn given_a_board_outside_the_box_when_drawn_then_the_box_and_its_shape_are_not_held() {
    // Given
    let view = status("", false, true);

    // When
    let svg = overlay(&view).unwrap_or_default();

    // Then
    assert!(svg.contains("class=\"target\" x=\"0\" y=\"0\" width=\"355\" height=\"237\""));
    assert!(svg.contains("class=\"shape\" points=\"29.5,19.8 325.5,19.8 325.5,217.2 29.5,217.2\""));
}

#[test]
fn given_a_board_inside_the_box_when_drawn_then_the_box_is_held_and_the_board_outlined() {
    // Given
    let view = status("", true, true);

    // When
    let svg = overlay(&view).unwrap_or_default();

    // Then
    assert!(svg.contains("class=\"target held\""));
    assert!(svg.contains("class=\"board\""));
    assert!(svg.contains("points=\"10.0,20.0 30.0,20.0 30.0,40.0 10.0,40.0\""));
}

#[test]
fn given_a_flat_position_when_the_line_is_written_then_it_says_to_hold_the_board_flat() {
    // Given
    let view = status("", false, true);

    // When
    let said = line(&view);

    // Then
    assert!(said.starts_with("Flat position 1 of 25: hold the board flat"));
}

#[test]
fn given_the_first_turned_position_when_the_line_is_written_then_it_names_the_side_to_turn() {
    // Given
    let mut view = status("left", false, false);
    view.position = 25;

    // When
    let said = line(&view);

    // Then
    assert!(said.starts_with(
        "Turned position 1 of 20: hold the board with its left side turned about 30 degrees away"
    ));
}

#[test]
fn given_a_flat_board_inside_a_turned_box_when_the_line_is_written_then_it_says_turn_it_now() {
    // Given
    let view = status("left", true, false);

    // When
    let said = line(&view);

    // Then
    assert!(said.starts_with("Inside the box: now turn the board's left side"));
}

#[test]
fn given_a_frame_that_counts_when_the_line_is_written_then_it_counts_the_frames_held() {
    // Given
    let view = status("", true, true);

    // When
    let said = line(&view);

    // Then
    assert!(said.starts_with("Hold it still: 3 of 10"));
}

#[test]
fn given_a_flat_position_when_the_turn_bar_is_read_then_there_is_none() {
    // Given
    let view = status("", true, true);

    // When
    let meter = turn(&view);

    // Then
    assert_eq!(meter, None);
}

#[test]
fn given_half_the_aimed_turn_when_the_turn_bar_is_read_then_it_is_half_full_and_marked() {
    // Given
    let mut view = status("left", true, false);
    view.turn_deg = Some(15.0);

    // When
    let meter = turn(&view);

    // Then
    assert_eq!(meter.as_ref().map(|meter| meter.fraction), Some(0.5));
    assert_eq!(meter.as_ref().map(|meter| meter.good), Some(false));
    assert_eq!(meter.and_then(|meter| meter.mark), Some(20.0 / 30.0));
}

#[test]
fn given_a_turn_past_the_least_when_the_turn_bar_is_read_then_it_is_good() {
    // Given
    let mut view = status("left", true, false);
    view.turn_deg = Some(24.0);

    // When
    let meter = turn(&view);

    // Then
    assert_eq!(meter.map(|meter| meter.good), Some(true));
}

#[test]
fn given_a_turn_the_wrong_way_when_the_turn_bar_is_read_then_it_says_so() {
    // Given
    let mut view = status("left", true, false);
    view.turn_deg = Some(-12.0);

    // When
    let meter = turn(&view);

    // Then
    assert!(meter.is_some_and(|meter| meter.label.contains("12 degrees the wrong way")));
}

#[test]
fn given_27_positions_done_when_the_progress_bar_is_read_then_it_counts_them_and_the_stills() {
    // Given
    let mut view = status("left", false, false);
    view.position = 27;

    // When
    let meter = progress(&view);

    // Then
    assert_eq!(meter.fraction, 27.0 / 45.0);
    assert!(
        meter
            .label
            .starts_with("27 of 45 positions done, 2 stills kept")
    );
}

#[test]
fn given_3_of_10_frames_held_when_the_hold_bar_is_read_then_it_is_three_tenths_full() {
    // Given
    let view = status("left", false, false);

    // When
    let meter = hold(&view);

    // Then
    assert_eq!(meter.fraction, 0.3);
}
