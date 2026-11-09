//! The settings `config/motion.toml` sets, compiled in by `build/motion.rs`.
//! `"/motion.rs"` must match that file's `SOURCE_FILE_NAME`: `include!` takes
//! only a plain string.

include!(concat!(env!("OUT_DIR"), "/motion.rs"));
