use crate::geometry::point::Point;

/// One of the pool's entries: a frame's age and what it showed. The age is 0
/// for the newest frame and negative for older ones: how long before the
/// newest capture this one happened, ms (`real-loop-dia.md`, *Contracts*, the
/// pool).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Entry {
    pub(crate) age_ms: f64,
    pub(crate) pivot: Point,
    pub(crate) end: Point,
}
