pub(crate) mod result;

// ---------------------------------------------------------------------------------------------- //

use error_location::ErrorLocation;
use wasm_bindgen::JsValue;

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
}

impl AppError {
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
