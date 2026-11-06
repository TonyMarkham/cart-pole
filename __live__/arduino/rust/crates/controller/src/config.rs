//! The values `config.toml` sets, compiled in by the build script.
//!
//! The two strings below must match `build.rs`:
//!
//! - `"OUT_DIR"` matches its `OUT_DIR` constant.
//! - `"/config.rs"` matches its `CONFIG_SOURCE_FILE_NAME` constant.
//!
//! They are written out because `include!` only accepts plain strings, not
//! constants.

include!(concat!(env!("OUT_DIR"), "/config.rs"));
