use crate::trainer_link::pool_entry::PoolEntry;

use serde::Serialize;

/// 17: the stdout line, one entry per instance advanced, in instance
/// order (synthetic-loop-dia.md, Training instances).
#[derive(Serialize)]
pub struct PoolsMessage<'a> {
    pub pools: &'a [PoolEntry<'a>],
}
