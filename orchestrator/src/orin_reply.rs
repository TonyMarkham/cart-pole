use serde::Deserialize;

/// Entries in the pool (synthetic-loop-dia.md, Contracts, the pool).
pub const POOL_ENTRIES: usize = 6;

/// One pool entry: its age in ms from the current capture (0 for the
/// newest, negative for older), then x0, y0, … x3, y3, the 4 markers in
/// class order, normalized, (−1, −1) where not detected.
pub type PoolRow = [f64; 9];

/// Synthetic Orin → orchestrator, exactly one text message per pose, in
/// order (synthetic-loop-dia.md, 14): `{"id": 7, "pool": [[age_ms, x0,
/// y0, …, x3, y3], …6 rows, newest first], "failed": false}`. `failed` is
/// true when 3 of the pool's 6 entries are failed frames: the run fails.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct OrinReply {
    pub id: u32,
    pub pool: Vec<PoolRow>,
    pub failed: bool,
}
