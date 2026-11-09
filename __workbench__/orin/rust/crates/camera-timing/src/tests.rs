//! Checks of gap 1's arithmetic and the patch, each against values worked
//! out by hand.

use crate::{
    constants::{FRAME_HEIGHT, FRAME_WIDTH},
    gap_one::GapOne,
    patch::Patch,
};

/// The LED lit no earlier than 100 ms and went out no later than 100.5 ms; a
/// 1 ms exposure that showed it started between 99 ms and 100.5 ms. The frame
/// arrived at 130 ms, so gap 1 lies between 130 − 100.5 = 29.5 ms and
/// 130 − 100 + 1 = 31 ms: middle 30.25 ms, ±0.75 ms.
#[test]
fn given_a_caught_flash_when_gap_one_is_worked_out_then_its_band_is_the_hand_worked_one() {
    let millis = 1e6;
    let gap = GapOne::of(130.0 * millis, 100.0 * millis, 100.5 * millis, 1.0 * millis);
    assert!((gap.low_ns - 29.5 * millis).abs() < 1e-6);
    assert!((gap.high_ns - 31.0 * millis).abs() < 1e-6);
    assert!((gap.mid_ns() - 30.25 * millis).abs() < 1e-6);
    assert!((gap.half_width_ns() - 0.75 * millis).abs() < 1e-6);
}

/// One pixel at row 10, column 20 gains 100 grey levels; with a half-width of
/// 2 the patch is columns 18..23 and rows 8..13, 25 pixels, and its level is
/// that pixel's 100 shared among them: 4.
#[test]
fn given_one_brighter_pixel_when_the_patch_is_found_then_it_is_centred_there() {
    let off = vec![0.0; FRAME_WIDTH * FRAME_HEIGHT];
    let mut on = off.clone();
    on[10 * FRAME_WIDTH + 20] = 100.0;
    let patch = Patch::find(&off, &on, 2);
    assert_eq!(
        patch,
        Patch {
            left: 18,
            right: 23,
            top: 8,
            bottom: 13
        }
    );
    assert!((patch.level(&on) - 4.0).abs() < 1e-12);
}

/// At the frame's top-left corner the patch is cut at the edges: columns
/// 0..3 and rows 0..3 for a half-width of 2.
#[test]
fn given_a_pixel_at_the_corner_when_the_patch_is_found_then_it_stops_at_the_edges() {
    let off = vec![0.0; FRAME_WIDTH * FRAME_HEIGHT];
    let mut on = off.clone();
    on[0] = 50.0;
    let patch = Patch::find(&off, &on, 2);
    assert_eq!(
        (patch.left, patch.right, patch.top, patch.bottom),
        (0, 3, 0, 3)
    );
}
