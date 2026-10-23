use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

/// `physics.toml` (repo root): where MuJoCo is, the model, and its MJCF
/// names. Paths are resolved against the file's folder.
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    /// The file this was read from.
    pub path: PathBuf,
    /// The MuJoCo release tree. Linked at build time; informational at
    /// run time (the build read the same file).
    pub mujoco_dir: PathBuf,
    /// The MJCF to load.
    pub model: PathBuf,
    pub names: Names,
}

/// MJCF names as in the model's `<joint name>` / actuator element
/// `name`. The joint names are also the wire keys of the player's state
/// message.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Names {
    pub cart_joint: String,
    pub pole_joint: String,
    pub actuator: String,
}

// The file as written; unknown keys are errors so a misspelt key cannot
// silently fall back.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    mujoco_dir: PathBuf,
    model: PathBuf,
    names: Names,
}

impl PhysicsConfig {
    /// Reads the `physics.toml` the build script linked MuJoCo from — its
    /// path is baked in at build time, so the current directory does not
    /// matter.
    pub fn load() -> Result<PhysicsConfig> {
        PhysicsConfig::read(Path::new(env!("PHYSICS_TOML")))
    }

    fn read(path: &Path) -> Result<PhysicsConfig> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let file: File = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let folder = path
            .parent()
            .ok_or_else(|| anyhow!("{} has no parent", path.display()))?;
        Ok(PhysicsConfig {
            path: path.to_path_buf(),
            mujoco_dir: folder.join(file.mujoco_dir),
            model: folder.join(file.model),
            names: file.names,
        })
    }
}
