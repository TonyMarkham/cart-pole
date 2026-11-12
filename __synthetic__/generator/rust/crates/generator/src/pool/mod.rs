//! The pool of generation sets: the master (`read_set`) and the sets folder,
//! one `*.json` per set (`load_pool`). Each set file holds only what differs
//! from the master and is laid over a copy of it (`GenerationSetBuilder`);
//! the merged sets are checked against the pool's rules (`check`) before
//! anything runs.

pub(crate) mod check;
pub(crate) mod generation_set_builder;
pub(crate) mod load_pool;
pub(crate) mod parse_set;
pub(crate) mod pool_entry;
pub(crate) mod read_set;

pub(crate) use crate::pool::{load_pool::load_pool, read_set::read_set};
