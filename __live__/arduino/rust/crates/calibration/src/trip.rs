use crate::{settings::TRIP_US, switch::Switch};

/// A switch counts as tripped only once it has read pressed without a
/// break for `trip_us`. A spike picked up on the switch wire, or the
/// contact chattering as the rig shakes, reads pressed for far less, and
/// is not a trip (one did halt the first jump-limit run, 2026-10-27,
/// 143 mm from the switch). A move reads a switch only when it wakes: at the
/// switch's press, which its edge interrupt wakes it for, at each 2 ms tick,
/// and when the wake timer ends at `due`, `trip_us` after the press. A
/// release and press again between two readings leaves the edge latch set,
/// so that break is seen even when both readings are pressed. So a trip is
/// confirmed `trip_us` after the press, plus the time to wake.
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

    /// Takes `switch`'s reading at `now` (clock cycles), and `edge`, whether
    /// it was pressed since the last reading: true once it has read pressed
    /// without a break for `trip_us`. An edge while pressed is a new press,
    /// so the time starts again.
    pub fn tripped(&mut self, switch: Switch, pressed: bool, edge: bool, now: u64) -> bool {
        let since = &mut self.since[switch.index()];
        if !pressed {
            *since = None;
            return false;
        }
        if edge {
            *since = Some(now);
        }
        now - *since.get_or_insert(now) >= self.hold
    }

    /// When the earliest pressed switch's `trip_us` is up, in clock cycles:
    /// the time to wake the move to read it again. None while neither switch
    /// reads pressed.
    pub fn due(&self) -> Option<u64> {
        self.since.iter().flatten().map(|since| since + self.hold).min()
    }
}
