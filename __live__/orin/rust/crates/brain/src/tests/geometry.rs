use crate::geometry::{
    homography::Homography, lens::Lens, lens_record::LensRecord, matrix_record::MatrixRecord,
};

use serde_json::json;
use std::path::Path;

/// 2026-11-02's lens, halved for 728 x 544 frames (`lens.py`'s header), with a
/// bend to undo.
fn bent_lens() -> Lens {
    Lens::from_record(&LensRecord {
        camera_matrix: vec![
            vec![923.5, 0.0, 339.6],
            vec![0.0, 923.5, 272.0],
            vec![0.0, 0.0, 1.0],
        ],
        dist: json!([-0.3, 0.1, 0.001, -0.002, 0.05]),
    })
    .unwrap()
}

#[test]
fn given_a_lens_with_no_bend_when_a_pixel_is_undone_then_it_stays_where_it_was() {
    // Given
    let lens = Lens::straight(923.5, 923.5, 339.6, 272.0);

    // When
    let (u, v) = lens.undo(100.0, 500.0);

    // Then
    assert!((u - 100.0).abs() < 1e-9 && (v - 500.0).abs() < 1e-9);
}

#[test]
fn given_a_bent_lens_when_a_bent_pixel_is_undone_then_the_straight_pixel_comes_back() {
    // Given: a point on the normalised plane, where a lens with no bend would
    // show it, and where this lens shows it
    let lens = bent_lens();
    let (x, y) = (0.3, -0.2);
    let straight = (923.5 * x + 339.6, 923.5 * y + 272.0);
    let bent = lens.bend(x, y);
    assert!(
        (bent.0 - straight.0).abs() > 1.0,
        "the test's lens must bend this point"
    );

    // When
    let undone = lens.undo(bent.0, bent.1);

    // Then
    assert!(
        (undone.0 - straight.0).abs() < 1e-9,
        "{undone:?} vs {straight:?}"
    );
    assert!(
        (undone.1 - straight.1).abs() < 1e-9,
        "{undone:?} vs {straight:?}"
    );
}

#[test]
fn given_nested_coefficients_when_the_lens_is_read_then_they_are_taken_as_the_flat_ones_would_be() {
    // Given
    let flat = bent_lens();
    let nested = LensRecord {
        camera_matrix: vec![
            vec![923.5, 0.0, 339.6],
            vec![0.0, 923.5, 272.0],
            vec![0.0, 0.0, 1.0],
        ],
        dist: json!([[-0.3, 0.1, 0.001, -0.002, 0.05]]),
    };

    // When
    let lens = Lens::from_record(&nested).unwrap();

    // Then
    assert_eq!(lens, flat);
}

#[test]
fn given_a_scale_and_shift_matrix_when_pixels_are_mapped_then_the_metres_follow_it() {
    // Given: a millimetre a pixel, the frame's centre at (0, 0), z up
    let matrix = [[0.001, 0.0, -0.364], [0.0, -0.001, 0.272], [0.0, 0.0, 1.0]];
    let homography = Homography::with_straight_lens(matrix, Lens::straight(1.0, 1.0, 0.0, 0.0));

    // When
    let centre = homography.to_metres(364.0, 272.0);
    let right = homography.to_metres(728.0, 272.0);
    let top = homography.to_metres(364.0, 0.0);

    // Then
    assert!(centre.x.abs() < 1e-12 && centre.z.abs() < 1e-12);
    assert!((right.x - 0.364).abs() < 1e-12 && right.z.abs() < 1e-12);
    assert!(top.x.abs() < 1e-12 && (top.z - 0.272).abs() < 1e-12);
}

#[test]
fn given_a_perspective_matrix_when_a_pixel_is_mapped_then_it_is_divided_by_the_third_row() {
    // Given
    let matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 2.0]];
    let homography = Homography::with_straight_lens(matrix, Lens::straight(1.0, 1.0, 0.0, 0.0));

    // When
    let point = homography.to_metres(10.0, 4.0);

    // Then
    assert!((point.x - 5.0).abs() < 1e-12 && (point.z - 2.0).abs() < 1e-12);
}

fn record(within_limit: bool, accepted: bool, frame: Vec<u64>) -> MatrixRecord {
    MatrixRecord {
        matrix: vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ],
        lens: LensRecord {
            camera_matrix: vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
            dist: json!([0.0, 0.0, 0.0, 0.0, 0.0]),
        },
        within_limit,
        accepted,
        frame,
    }
}

#[test]
fn given_a_fit_over_the_limit_and_not_accepted_when_loaded_then_it_is_refused_with_what_to_do() {
    // Given
    let record = record(false, false, vec![728, 544]);

    // When
    let loaded = Homography::from_record(&record, Path::new("homography.json"));

    // Then
    let message = loaded.unwrap_err().to_string();
    assert!(
        message.contains("did not pass and was not accepted"),
        "{message}"
    );
}

#[test]
fn given_a_fit_over_the_limit_but_accepted_when_loaded_then_it_is_taken() {
    assert!(Homography::from_record(&record(false, true, vec![728, 544]), Path::new("h")).is_ok());
}

#[test]
fn given_a_fit_on_full_frames_when_loaded_then_it_is_refused() {
    // Given
    let record = record(true, false, vec![1456, 1088]);

    // When
    let loaded = Homography::from_record(&record, Path::new("homography.json"));

    // Then
    assert!(
        loaded
            .unwrap_err()
            .to_string()
            .contains("fitted on [1456, 1088] frames")
    );
}
