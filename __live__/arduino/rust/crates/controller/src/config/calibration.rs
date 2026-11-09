//! The settings `config/calibration.toml` sets, compiled in by
//! `build/calibration.rs`. `"/calibration.rs"` must match that file's
//! `SOURCE_FILE_NAME`: `include!` takes only a plain string.

include!(concat!(env!("OUT_DIR"), "/calibration.rs"));
