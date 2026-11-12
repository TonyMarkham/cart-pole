use error_location::ErrorLocation;
use std::panic::Location;

/// Why generate-csharp stopped, in words for the terminal, each with where in
/// the code it was raised. The message names the operation and the file,
/// URL or entry it failed on.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    /// A file or folder could not be read or written.
    #[error("[IO Error]: {message}{location}")]
    Io {
        message: String,
        location: ErrorLocation,
    },

    /// Google.Protobuf.Tools could not be downloaded, does not match its
    /// pinned SHA-256, or lacks the protoc this machine needs.
    #[error("[Tools Error]: {message}{location}")]
    Tools {
        message: String,
        location: ErrorLocation,
    },

    /// protoc did not start or did not succeed.
    #[error("[Protoc Error]: {message}{location}")]
    Protoc {
        message: String,
        location: ErrorLocation,
    },

    /// protoc's GenerationSet.cs is not in the shape the lifecycle file is
    /// read from (no namespace, no classes).
    #[error("[Generated Code Error]: {message}{location}")]
    Generated {
        message: String,
        location: ErrorLocation,
    },
}

impl AppError {
    #[track_caller]
    pub(crate) fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn tools(message: impl Into<String>) -> Self {
        Self::Tools {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn protoc(message: impl Into<String>) -> Self {
        Self::Protoc {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn generated(message: impl Into<String>) -> Self {
        Self::Generated {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
