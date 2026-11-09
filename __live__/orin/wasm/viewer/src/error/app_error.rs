use error_location::ErrorLocation;
use wasm_bindgen::JsValue;

/// What went wrong in the page: shown on it when it is the webserver's own
/// words, otherwise written to the browser's console with where it was raised.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
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
    /// The webserver's own words, refusing a request.
    #[error("{0}")]
    Answer(String),
}

impl AppError {
    pub(crate) fn answer(words: String) -> Self {
        Self::Answer(words)
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
