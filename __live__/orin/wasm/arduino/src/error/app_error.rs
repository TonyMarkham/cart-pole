use error_location::ErrorLocation;
use wasm_bindgen::JsValue;

/// What went wrong in the page: shown on it when it is the webserver's own
/// words, otherwise written to the browser's console with where it was raised.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    /// Shown on the page as it stands, so it carries no source location.
    #[error("{message}")]
    Answer { message: String },
    #[error("[Browser Error]: {message}{location}")]
    Browser {
        message: String,
        location: ErrorLocation,
    },
    #[error("[Missing Browser Object]: {name}{location}")]
    Missing {
        name: String,
        location: ErrorLocation,
    },
}

impl AppError {
    /// What the webserver said when it had no reading: its own words, for the
    /// page to show.
    pub(crate) fn answer(message: String) -> Self {
        Self::Answer { message }
    }

    #[track_caller]
    pub(crate) fn missing(name: &str) -> Self {
        Self::Missing {
            name: name.to_string(),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}

impl From<JsValue> for AppError {
    #[track_caller]
    fn from(error: JsValue) -> Self {
        Self::Browser {
            message: format!("{error:?}"),
            location: ErrorLocation::from(std::panic::Location::caller()),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::answer(error.to_string())
    }
}
