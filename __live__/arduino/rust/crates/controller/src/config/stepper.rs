//! The settings `config/stepper.toml` sets, compiled in by `build/stepper.rs`.
//! `"/stepper.rs"` must match that file's `SOURCE_FILE_NAME`: `include!` takes
//! only a plain string.

include!(concat!(env!("OUT_DIR"), "/stepper.rs"));
