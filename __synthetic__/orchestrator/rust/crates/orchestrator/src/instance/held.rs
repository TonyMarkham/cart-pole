use crate::{
    Capture,
    orin::{derived_state::DerivedState, orin_reply::PoolRow},
};

/// A captured step waiting for its send (__documentation__/synthetic-loop-dia.md, the loop,
/// 11–14): its id, capture state, and the pool and five numbers its Orin
/// replied with, and how long that reply took, ms of the PC's time.
pub struct Held {
    pub step: i64,
    pub id: u32,
    pub capture: Capture,
    pub pool: Vec<PoolRow>,
    pub state: DerivedState,
    pub orin_ms: f64,
}
