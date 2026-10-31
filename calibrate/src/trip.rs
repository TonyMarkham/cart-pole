use crate::{settings::TRIP_US, switch::Switch};

/// A switch counts as tripped only once it has read pressed without a
/// break for `trip_us`. A spike picked up on the switch wire, or the
/// contact chattering as the rig shakes, reads pressed for far less, and
/// is not a trip (one did halt the first jump-limit run, 2026-10-27,
/// 143 mm from the switch). The wait is the same at every real trip, so
/// it cancels in the homing's span and in the experiment's noise floor,
/// as the RC filter the design first had would have (control-design.md,
/// *The limit switches*). The switches are read once a pulse, so a trip
/// takes at least two readings that far apart.
pub struct TripFilter {
    /// When each switch began reading pressed, in clock cycles.
    since: [Option<u64>; 2],
    /// `trip_us` in clock cycles.
    hold: u64,
}

impl TripFilter {
    /// `per_us` is clock cycles per µs (ICLK ÷ 1,000,000).
    pub fn new(per_us: u64) -> Self {
        Self {
            since: [None; 2],
            hold: u64::from(TRIP_US) * per_us,
        }
    }

    /// Takes `switch`'s reading at `now` (clock cycles): true once it has
    /// read pressed at every reading for `trip_us`.
    pub fn tripped(&mut self, switch: Switch, pressed: bool, now: u64) -> bool {
        let since = &mut self.since[switch.index()];
        if !pressed {
            *since = None;
            return false;
        }
        now - *since.get_or_insert(now) >= self.hold
    }
}
