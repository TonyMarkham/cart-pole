use crate::error::{app_error::AppError, app_result::AppResult};

use serde::Deserialize;
use std::{fs, path::Path};

/// protobuf-tools.toml (__synthetic__/generator/rust/): which
/// Google.Protobuf.Tools release supplies protoc, and its .nupkg's SHA-256.
/// The release must be the same as the Unity package's
/// Runtime/Plugins/Google.Protobuf.dll; the toml's comments say how to bump
/// both.
#[derive(Deserialize, Debug)]
pub(crate) struct ToolsPin {
    /// The release, e.g. "3.36.2".
    pub(crate) version: String,
    /// The SHA-256 of that release's .nupkg, lowercase hex.
    pub(crate) sha256: String,
}

impl ToolsPin {
    pub(crate) fn load(path: &Path) -> AppResult<Self> {
        let text = fs::read_to_string(path)
            .map_err(|error| AppError::io(format!("reading {}: {error}", path.display())))?;
        toml::from_str(&text)
            .map_err(|error| AppError::tools(format!("parsing {}: {error}", path.display())))
    }
}
