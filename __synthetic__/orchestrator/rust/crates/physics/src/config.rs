use crate::names::Names;

use std::path::PathBuf;

/// What a run loads: the MJCF and the names in it. The caller supplies
/// both (the orchestrator takes them from its command line, which the
/// trainer fills from its settings.toml [mjcf]); `orchestrator.toml` only
/// says which MuJoCo the build links.
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    /// The MJCF to load.
    pub model: PathBuf,
    pub names: Names,
}
