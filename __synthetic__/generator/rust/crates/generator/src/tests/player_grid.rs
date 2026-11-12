//! The players' window grid from rows, columns and the screen size.

use crate::players::player_grid::PlayerGrid;

use std::num::NonZeroU16;

/// A `NonZeroU16` for the test values below, all non-zero.
fn n(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).unwrap()
}

#[test]
fn given_the_trainers_grid_when_built_then_12_players_get_960_by_704_windows() {
    // given: instance_rows 3, instance_columns 4, screen 3840 x 2112

    // when
    let grid = PlayerGrid::new(n(3), n(4), n(3_840), n(2_112)).unwrap();

    // then
    assert_eq!(grid.players, 12);
    assert_eq!(grid.columns.get(), 4);
    assert_eq!(grid.window.width.get(), 960);
    assert_eq!(grid.window.height.get(), 704);
}

#[test]
fn given_a_width_that_does_not_divide_when_built_then_the_window_rounds_down() {
    // given: 1000 pixels in 3 columns is 333.3

    // when
    let grid = PlayerGrid::new(n(1), n(3), n(1_000), n(600)).unwrap();

    // then
    assert_eq!(grid.window.width.get(), 333);
}

#[test]
fn given_more_columns_than_pixels_when_built_then_it_is_an_error() {
    // given: 4 pixels across 5 columns leaves windows 0 pixels wide

    // when
    let grid = PlayerGrid::new(n(1), n(5), n(4), n(600));

    // then
    assert!(grid.is_err());
}

#[test]
fn given_more_than_65535_players_when_built_then_it_is_an_error() {
    // given: 256 x 256 = 65536 players

    // when
    let grid = PlayerGrid::new(n(256), n(256), n(u16::MAX), n(u16::MAX));

    // then
    assert!(grid.is_err());
}
