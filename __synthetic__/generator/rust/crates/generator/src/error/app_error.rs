use error_location::ErrorLocation;
use std::panic::Location;

/// Why the generator stopped, in words for the terminal, each with where in
/// the code it was raised. The message names the operation and the file or
/// folder it failed on, e.g. with no sets folder: "\[IO Error\]: reading
/// __synthetic__/generator/rust\\../sets: The system cannot find the path
/// specified. (os error 3)\[...\\src\\pool\\load_pool.rs:16:41\]".
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    /// A file or folder could not be read.
    #[error("[IO Error]: {message}{location}")]
    Io {
        message: String,
        location: ErrorLocation,
    },

    /// generator.toml is not valid toml, or misses a setting.
    #[error("[Settings Error]: {message}{location}")]
    Settings {
        message: String,
        location: ErrorLocation,
    },

    /// A generation set's JSON does not match generation_set.proto.
    #[error("[Set Error]: {message}{location}")]
    Set {
        message: String,
        location: ErrorLocation,
    },

    /// The sets parse but break a rule of the pool (pool::check).
    #[error("[Pool Error]: {message}{location}")]
    Pool {
        message: String,
        location: ErrorLocation,
    },

    /// A player could not be launched or reached, broke the protocol, exited,
    /// or answered a job with an error.
    #[error("[Player Error]: {message}{location}")]
    Player {
        message: String,
        location: ErrorLocation,
    },

    /// The players' runs do not make one dataset: a markers.json that does
    /// not parse, or two runs whose markers disagree.
    #[error("[Dataset Error]: {message}{location}")]
    Dataset {
        message: String,
        location: ErrorLocation,
    },
}

impl AppError {
    #[track_caller]
    pub(crate) fn player(message: impl Into<String>) -> Self {
        Self::Player {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn dataset(message: impl Into<String>) -> Self {
        Self::Dataset {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn settings(message: impl Into<String>) -> Self {
        Self::Settings {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn set(message: impl Into<String>) -> Self {
        Self::Set {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn pool(message: impl Into<String>) -> Self {
        Self::Pool {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
