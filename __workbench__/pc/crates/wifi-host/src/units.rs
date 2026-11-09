//! Conversions between units, and counts, by definition.

pub(crate) const NANOS_PER_MICRO: f64 = 1e3;
pub(crate) const NANOS_PER_SECOND: f64 = 1e9;

/// A share as a percentage, and as parts per million.
pub(crate) const PERCENT: f64 = 100.0;
pub(crate) const PARTS_PER_MILLION: f64 = 1e6;

/// A midpoint is the mean of a pair of stamps: their sum over this.
pub(crate) const PAIR: f64 = 2.0;

/// A round trip is two ways, out and back: one way is the round trip over
/// this, when both ways take as long.
pub(crate) const WAYS_PER_ROUND_TRIP: f64 = 2.0;
