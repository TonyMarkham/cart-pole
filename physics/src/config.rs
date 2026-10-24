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

/// MJCF names as in the model's `<joint name>` / actuator element
/// `name`. The joint names are also the wire keys of the player's state
/// message.
#[derive(Debug, Clone)]
pub struct Names {
    pub cart_joint: String,
    pub pole_joint: String,
    pub actuator: String,
}
