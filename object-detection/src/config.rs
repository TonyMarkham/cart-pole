use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

use crate::SLOTS;

/// `ObjectDetection.toml`: the model and how its output becomes slots.
/// Paths are resolved against the file's folder. The caller gives the
/// file's path at run time; it is not baked in at build time like
/// `physics.toml`'s.
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// The file this was read from.
    pub path: PathBuf,
    /// The float ONNX model (tract, the default build).
    pub model: PathBuf,
    /// The same detector as an int8 Edge TPU model (the `edgetpu` build).
    pub edgetpu_model: PathBuf,
    /// The model's square input size, pixels.
    pub input_size: usize,
    /// Letterbox padding grey level, 0–255.
    pub pad_value: u8,
    /// A class's best score counts as detected at or above this.
    pub confidence: f32,
    /// Class names in class-id order = slot order.
    pub classes: Vec<String>,
    /// CPU threads inference runs on; 1 runs without a thread pool.
    pub threads: usize,
}

// The file as written; unknown keys are errors so a misspelt key cannot
// silently fall back.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    model: PathBuf,
    edgetpu_model: PathBuf,
    input_size: usize,
    pad_value: u8,
    confidence: f32,
    classes: Vec<String>,
    threads: usize,
}

impl DetectorConfig {
    /// Reads and checks `ObjectDetection.toml` at `path`.
    pub fn load(path: &Path) -> Result<DetectorConfig> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let file: File = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let folder = path
            .parent()
            .ok_or_else(|| anyhow!("{} has no parent", path.display()))?;
        if file.input_size == 0 {
            bail!("{}: input_size must be positive", path.display());
        }
        if file.threads == 0 {
            bail!("{}: threads must be at least 1", path.display());
        }
        if !(0.0..=1.0).contains(&file.confidence) {
            bail!("{}: confidence {} is not within 0..1", path.display(), file.confidence);
        }
        // The marker frame has one slot per class, and the contract
        // (loop-plan.md) fixes four.
        if file.classes.len() != SLOTS {
            bail!(
                "{}: {} classes, but the marker frame has {SLOTS} slots",
                path.display(),
                file.classes.len()
            );
        }
        Ok(DetectorConfig {
            path: path.to_path_buf(),
            model: folder.join(file.model),
            edgetpu_model: folder.join(file.edgetpu_model),
            input_size: file.input_size,
            pad_value: file.pad_value,
            confidence: file.confidence,
            classes: file.classes,
            threads: file.threads,
        })
    }
}
