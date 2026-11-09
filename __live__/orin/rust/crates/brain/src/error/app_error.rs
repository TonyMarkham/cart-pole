use error_location::ErrorLocation;

/// Why a step failed, in words for the terminal and for `/run`. Each carries
/// where in the code it was raised. A run that fails with one ends idle, with
/// the words kept as the brain's `message`.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    #[error("[Camera Error]: {message}{location}")]
    Camera {
        message: String,
        location: ErrorLocation,
    },

    #[error("[Homography Error]: {message}{location}")]
    Homography {
        message: String,
        location: ErrorLocation,
    },

    #[error("[IO Error]: {message}{location}")]
    Io {
        message: String,
        location: ErrorLocation,
    },

    #[error("[JSON Error]: {message}{location}")]
    Json {
        message: String,
        location: ErrorLocation,
    },

    #[error("[Settings Error]: {message}{location}")]
    Settings {
        message: String,
        location: ErrorLocation,
    },

    #[error("[Worker Error]: {message}{location}")]
    Worker {
        message: String,
        location: ErrorLocation,
    },
}

impl AppError {
    #[track_caller]
    pub(crate) fn camera(message: impl Into<String>) -> Self {
        Self::Camera {
            message: message.into(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn homography(message: impl Into<String>) -> Self {
        Self::Homography {
            message: message.into(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn settings(message: impl Into<String>) -> Self {
        Self::Settings {
            message: message.into(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn worker(message: impl Into<String>) -> Self {
        Self::Worker {
            message: message.into(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }

    /// The words alone, for `/run` to show: the place in the code is for the
    /// terminal.
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Camera { message, .. }
            | Self::Homography { message, .. }
            | Self::Io { message, .. }
            | Self::Json { message, .. }
            | Self::Settings { message, .. }
            | Self::Worker { message, .. } => message,
        }
    }
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

impl From<serde_json::Error> for AppError {
    #[track_caller]
    fn from(error: serde_json::Error) -> Self {
        Self::Json {
            message: error.to_string(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}
