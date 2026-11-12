use crate::proto::GenerationSet;

use std::path::PathBuf;

/// One set of the pool: the file it was read from (named in every error
/// about it), the set merged over the master, and whether its run is val
/// (its file name is in generator.toml's `val`) or train.
#[derive(Debug)]
pub(crate) struct PoolEntry {
    pub(crate) path: PathBuf,
    pub(crate) set: GenerationSet,
    pub(crate) is_val: bool,
}
