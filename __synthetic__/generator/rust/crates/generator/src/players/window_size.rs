use std::num::NonZeroU16;

/// A player window's size in pixels: one cell of the `PlayerGrid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowSize {
    pub(crate) width: NonZeroU16,
    pub(crate) height: NonZeroU16,
}
