use error_location::ErrorLocation;
use tokio::sync::watch;

/// Why the program stopped, in words for the terminal. Each carries where in
/// the code it was raised.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    #[error("[Image Channel Error]: {message}{location}")]
    ImageChannel {
        message: String,
        location: ErrorLocation,
    },

    #[error("[IO Error]: {message}{location}")]
    Io {
        message: String,
        location: ErrorLocation,
    },

    #[error("[Settings Error]: {message}{location}")]
    Settings {
        message: String,
        location: ErrorLocation,
    },

    #[error("[WebSocket Error]: {message}{location}")]
    WebSocket {
        message: String,
        location: ErrorLocation,
    },
}

impl From<toml::de::Error> for AppError {
    #[track_caller]
    fn from(error: toml::de::Error) -> Self {
        Self::Settings {
            message: error.to_string(),
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

impl From<watch::error::RecvError> for AppError {
    #[track_caller]
    fn from(error: watch::error::RecvError) -> Self {
        Self::ImageChannel {
            message: error.to_string(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}

impl From<axum::Error> for AppError {
    #[track_caller]
    fn from(error: axum::Error) -> Self {
        Self::WebSocket {
            message: error.to_string(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}
