//! `config/wifi_timing.toml`: how long the board waits on its WiFi chip, and
//! how many times it tries.

use crate::toml_value;

use anyhow::{Context, Result};
use std::{fmt::Write as _, fs, path::Path};

/// The settings file in `config/`, and the file written into `OUT_DIR`, which
/// `src/config/wifi_timing.rs` includes by the same name.
const FILE_NAME: &str = "wifi_timing.toml";
const SOURCE_FILE_NAME: &str = "wifi_timing.rs";

/// Each key, the constant it becomes, and that constant's doc.
const KEYS: [(&str, &str, &str); 14] = [
    (
        "frame_timeout_ms",
        "FRAME_TIMEOUT_MS",
        "A datagram's payload once its header has arrived, ms",
    ),
    (
        "drain_quiet_ms",
        "DRAIN_QUIET_MS",
        "Quiet that ends a drain, ms",
    ),
    (
        "reply_timeout_ms",
        "REPLY_TIMEOUT_MS",
        "A command's reply, ms",
    ),
    (
        "handshake_tries",
        "HANDSHAKE_TRIES",
        "Tries of the first `AT`",
    ),
    (
        "handshake_timeout_ms",
        "HANDSHAKE_TIMEOUT_MS",
        "Each `AT`'s reply, ms",
    ),
    (
        "handshake_pause_ms",
        "HANDSHAKE_PAUSE_MS",
        "Between `AT` tries, ms",
    ),
    (
        "soft_reset_tries",
        "SOFT_RESET_TRIES",
        "Tries of clearing WiFi state",
    ),
    (
        "soft_reset_timeout_ms",
        "SOFT_RESET_TIMEOUT_MS",
        "Each clearing's reply, ms",
    ),
    (
        "wifi_mode_timeout_ms",
        "WIFI_MODE_TIMEOUT_MS",
        "A WiFi mode change's reply, ms",
    ),
    (
        "begin_sta_timeout_ms",
        "BEGIN_STA_TIMEOUT_MS",
        "Starting the join's reply, ms",
    ),
    (
        "status_tries",
        "STATUS_TRIES",
        "Asks whether the join finished",
    ),
    (
        "status_pause_ms",
        "STATUS_PAUSE_MS",
        "Before each of those asks, ms",
    ),
    (
        "address_tries",
        "ADDRESS_TRIES",
        "Asks for the router's address",
    ),
    (
        "address_pause_ms",
        "ADDRESS_PAUSE_MS",
        "Between those asks, ms",
    ),
];

/// Reads `config/wifi_timing.toml` and writes its constants into `out_dir`.
pub fn generate(config_dir: &Path, out_dir: &Path) -> Result<()> {
    let path = config_dir.join(FILE_NAME);
    let table = toml_value::read(&path)?;

    let mut constants = String::new();
    for (key, name, doc) in KEYS {
        let value = toml_value::whole(&table, key, &path)?;
        writeln!(
            constants,
            "/// {doc}: `{key}`.\npub const {name}: u32 = {value};"
        )?;
    }

    let source = out_dir.join(SOURCE_FILE_NAME);
    fs::write(&source, constants).with_context(|| format!("writing {}", source.display()))
}
