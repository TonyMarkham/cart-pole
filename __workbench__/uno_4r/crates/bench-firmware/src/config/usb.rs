//! The settings `config/usb.toml` sets, compiled in by `build/usb.rs`.
//! `"/usb.rs"` must match that file's `SOURCE_FILE_NAME`: `include!` takes
//! only a plain string.

include!(concat!(env!("OUT_DIR"), "/usb.rs"));
