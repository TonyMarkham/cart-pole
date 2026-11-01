use crate::derived_state::DerivedState;

use serde::Deserialize;

/// Entries in the pool (synthetic-loop-dia.md, Contracts, the pool).
pub const POOL_ENTRIES: usize = 6;

/// One pool entry: its age in ms from the current capture (0 for the
/// newest, negative for older), then Pivot's and End's (x, z) in metres in
/// the pole's plane, (−1, −1) where not detected.
pub type PoolRow = [f64; 5];

/// Synthetic Orin → orchestrator, exactly one text message per pose, in
/// order (synthetic-loop-dia.md, 14): `{"id": 7, "pool": [[age_ms, pivot_x,
/// pivot_z, end_x, end_z], …6 rows, newest first], "failed": false, "state":
/// {"cart": x, "cos_theta": c, "sin_theta": s, "cart_speed": v, "pole_speed":
/// w}}`. `failed` is true when 3 of the pool's 6 entries are failed frames: the
/// run fails. `state` is this frame's five numbers, null where not seen.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct OrinReply {
    pub id: u32,
    pub pool: Vec<PoolRow>,
    pub failed: bool,
    pub state: DerivedState,
}
