use crate::sync_exchange::SyncExchange;

/// One sync as the fit sees it.
pub(crate) struct SyncPoint {
    pub(crate) exchange: SyncExchange,
    /// The R4's midpoint, its two stamps' mean, as a wide count.
    pub(crate) r4_mid: f64,
    /// The round trip over USB, ns (`SyncExchange::round_trip_ns`).
    pub(crate) round_trip_ns: f64,
    /// Whether the fit used it: among the shortest round trips of its window.
    pub(crate) kept: bool,
}
