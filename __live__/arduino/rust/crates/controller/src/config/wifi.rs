//! The settings `config/wifi.toml` and its credentials file set, compiled in
//! by `build/wifi.rs`. `"/wifi.rs"` must match that file's `SOURCE_FILE_NAME`:
//! `include!` takes only a plain string.

include!(concat!(env!("OUT_DIR"), "/wifi.rs"));
