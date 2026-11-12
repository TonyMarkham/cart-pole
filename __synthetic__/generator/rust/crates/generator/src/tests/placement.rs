//! Each player's port and window place.

use crate::players::{
    placement::{player_port, window_position},
    window_size::WindowSize,
};

use std::num::NonZeroU16;

/// One cell of generator.toml's grid: 3840 x 2112 in 3 rows of 4.
fn window_size() -> WindowSize {
    WindowSize {
        width: NonZeroU16::new(960).unwrap(),
        height: NonZeroU16::new(704).unwrap(),
    }
}

/// generator.toml's instance_columns.
fn columns() -> NonZeroU16 {
    NonZeroU16::new(4).unwrap()
}

#[test]
fn given_first_port_9101_when_player_2_asks_then_it_gets_9103() {
    // given
    let first_port = 9101;

    // when
    let port = player_port(first_port, 2);

    // then
    assert_eq!(port.unwrap(), 9103);
}

#[test]
fn given_the_last_port_when_player_1_asks_then_it_is_an_error() {
    // given
    let first_port = u16::MAX;

    // when
    let port = player_port(first_port, 1);

    // then
    assert!(port.is_err());
}

#[test]
fn given_four_columns_when_player_5_is_placed_then_it_is_column_1_row_1() {
    // given
    let size = window_size();

    // when
    let position = window_position(5, columns(), size);

    // then
    assert_eq!(position, (960, 704));
}

#[test]
fn given_four_columns_when_player_11_is_placed_then_it_is_the_bottom_right_cell() {
    // given
    let size = window_size();

    // when
    let position = window_position(11, columns(), size);

    // then: column 3, row 2
    assert_eq!(position, (2880, 1408));
}

#[test]
fn given_four_columns_when_player_0_is_placed_then_it_is_the_top_left_corner() {
    // given
    let size = window_size();

    // when
    let position = window_position(0, columns(), size);

    // then
    assert_eq!(position, (0, 0));
}
