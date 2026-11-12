use crate::{
    error::{app_error::AppError, app_result::AppResult},
    players::window_size::WindowSize,
};

use std::num::NonZeroU16;

/// The players' windows as a grid filling the screen area, the policy
/// trainer's layout (__synthetic__/trainer/python/settings.toml,
/// `instance_rows`, `instance_columns`, `screen_width`, `screen_height`;
/// __documentation__/synthetic-loop-dia.md, Training instances, The
/// windows): `rows x columns` players, each window `screen / grid` pixels.
/// Example: 3 rows of 4 on 3840 x 2112 is 12 players of 960 x 704.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerGrid {
    /// Windows per row; player k sits in column `k mod columns`, row
    /// `k / columns`.
    pub(crate) columns: NonZeroU16,
    /// `rows x columns`: how many players run at once.
    pub(crate) players: u16,
    /// Each window's size, and the grid's spacing.
    pub(crate) window: WindowSize,
}

impl PlayerGrid {
    /// The grid of `rows x columns` windows filling `screen_width x
    /// screen_height`. Rounding down can leave fewer than `columns` pixels
    /// at the right and `rows` at the bottom. Fails when the player count
    /// passes 65535 or a window would be under 1 pixel.
    pub(crate) fn new(
        rows: NonZeroU16,
        columns: NonZeroU16,
        screen_width: NonZeroU16,
        screen_height: NonZeroU16,
    ) -> AppResult<Self> {
        let players = rows.get().checked_mul(columns.get()).ok_or_else(|| {
            AppError::settings(format!(
                "instance_rows {rows} x instance_columns {columns} is more than {} players",
                u16::MAX
            ))
        })?;
        let width = NonZeroU16::new(screen_width.get() / columns.get());
        let height = NonZeroU16::new(screen_height.get() / rows.get());
        let (Some(width), Some(height)) = (width, height) else {
            return Err(AppError::settings(format!(
                "a {screen_width} x {screen_height} screen in {rows} rows and {columns} columns \
                 leaves windows under 1 pixel"
            )));
        };
        Ok(Self {
            columns,
            players,
            window: WindowSize { width, height },
        })
    }
}
