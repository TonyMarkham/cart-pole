//! The pool: the policy's memory of what the camera saw, the last 6 frames'
//! points with their ages. The rules are the synthetic Orin's
//! (`__synthetic__/orin/python/src/synthetic_orin.py`, the rules), copied here
//! because live and synthetic share no code (`AGENTS.md`).

pub(crate) mod entry;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{ENTRIES, NANOS_PER_MILLI, POOL_FLOATS, ROW},
    geometry::point::Point,
    pool::entry::Entry,
};

use std::collections::VecDeque;

/// Six entries, newest first. Every push ages the older entries by the time
/// between the two newest captures, GStreamer's stamps, and drops the oldest.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pool {
    entries: VecDeque<Entry>,
    /// The newest capture's stamp, once a frame has been pushed.
    last_stamp_ns: Option<u64>,
}

impl Pool {
    /// The pool before any frame: six entries with no detection, `gap_ms`
    /// apart, the newest `gap_ms` before the first frame. So the first frame's
    /// ages come out as 0, −gap, −2 gap, …, −5 gap (the synthetic Orin's
    /// `pre_warm`, with every gap the same).
    pub(crate) fn pre_warmed(gap_ms: f64) -> Pool {
        let entries = (1..=ENTRIES)
            .map(|k| Entry {
                age_ms: -(k as f64) * gap_ms,
                pivot: Point::NOT_DETECTED,
                end: Point::NOT_DETECTED,
            })
            .collect();
        Pool {
            entries,
            last_stamp_ns: None,
        }
    }

    /// A frame's points, at age 0. Every older entry ages by this capture's
    /// stamp less the last one's; before the first frame the pre-warmed ages
    /// already count from it, so nothing ages. The oldest entry drops.
    pub(crate) fn push(&mut self, stamp_ns: u64, pivot: Point, end: Point) {
        if let Some(last) = self.last_stamp_ns {
            let aged_ms = (stamp_ns as f64 - last as f64) / NANOS_PER_MILLI;
            for entry in &mut self.entries {
                entry.age_ms -= aged_ms;
            }
        }
        self.last_stamp_ns = Some(stamp_ns);
        self.entries.push_front(Entry {
            age_ms: 0.0,
            pivot,
            end,
        });
        self.entries.truncate(ENTRIES);
    }

    /// The entries, newest first, for the tests; the loop reads `flat`.
    #[cfg(test)]
    pub(crate) fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// The pool as the policy reads it: per entry newest first, its age, then
    /// Pivot's (x, z) and End's (x, z): 30 numbers.
    pub(crate) fn flat(&self) -> [f64; POOL_FLOATS] {
        let mut values = [0f64; POOL_FLOATS];
        for (k, entry) in self.entries.iter().enumerate() {
            values[k * ROW] = entry.age_ms;
            values[k * ROW + 1] = entry.pivot.x;
            values[k * ROW + 2] = entry.pivot.z;
            values[k * ROW + 3] = entry.end.x;
            values[k * ROW + 4] = entry.end.z;
        }
        values
    }
}
