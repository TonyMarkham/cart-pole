//! The settings `config/wifi_timing.toml` sets, compiled in by
//! `build/wifi_timing.rs`. `"/wifi_timing.rs"` must match that file's
//! `SOURCE_FILE_NAME`: `include!` takes only a plain string.

include!(concat!(env!("OUT_DIR"), "/wifi_timing.rs"));
