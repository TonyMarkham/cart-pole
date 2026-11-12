use crate::{
    error::{app_error::AppError, app_result::AppResult},
    players::window_size::WindowSize,
};

use std::num::NonZeroU16;

/// Player `k`'s port: `first_port + k`. Example: first_port 9101, player 2
/// listens on 9103. Fails when that would pass the last port, 65535.
pub(crate) fn player_port(first_port: u16, k: u16) -> AppResult<u16> {
    first_port.checked_add(k).ok_or_else(|| {
        AppError::settings(format!(
            "first_port {first_port} + player {k} is past the last port, {}",
            u16::MAX
        ))
    })
}

/// Player `k`'s window's top-left corner in pixels from the screen's: column
/// `k mod columns`, row `k / columns`, each window `size` apart. Example: 3
/// columns of 640 x 480 windows put player 4 at column 1, row 1: (640, 480).
/// Each product is under 65536 x 65536, so it fits a `u32`.
pub(crate) fn window_position(k: u16, columns: NonZeroU16, size: WindowSize) -> (u32, u32) {
    let column = u32::from(k % columns.get());
    let row = u32::from(k / columns.get());
    (
        column.saturating_mul(u32::from(size.width.get())),
        row.saturating_mul(u32::from(size.height.get())),
    )
}
