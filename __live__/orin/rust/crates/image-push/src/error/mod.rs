pub(crate) mod result;

// ---------------------------------------------------------------------------------------------- //

use error_location::ErrorLocation;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    #[error("[HTTP Error]: {message}{location}")]
    Http {
        message: String,
        location: ErrorLocation,
    },

    #[error("[IO Error]: {message}{location}")]
    Io {
        message: String,
        location: ErrorLocation,
    },

    #[error("[Not A PNG]: {message}{location}")]
    NotPng {
        message: String,
        location: ErrorLocation,
    },
}

impl AppError {
    #[track_caller]
    pub(crate) fn not_png(path: &Path) -> Self {
        Self::NotPng {
            message: path.display().to_string(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}

impl From<std::io::Error> for AppError {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}

impl From<ureq::Error> for AppError {
    #[track_caller]
    fn from(error: ureq::Error) -> Self {
        Self::Http {
            message: error.to_string(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}
