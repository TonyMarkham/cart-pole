//! `config/usb.toml`: the settings the USB serial line to the Pi is opened
//! with.

use crate::toml_value;

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

/// The settings file in `config/`, and the file written into `OUT_DIR`, which
/// `src/config/usb.rs` includes by the same name.
const FILE_NAME: &str = "usb.toml";
const SOURCE_FILE_NAME: &str = "usb.rs";

const BAUD_KEY: &str = "baud";

/// The two rates at which the board's USB bridge resets the processor instead
/// of only setting the speed (arduino-manager.toml, [serial] baud, citing the
/// bridge's UNOR4USBBridge.ino, usbEventCallback).
const RESET_BAUDS: [u32; 2] = [1_200, 2_400];

/// Reads `config/usb.toml` and writes its constants into `out_dir`.
pub fn generate(config_dir: &Path, out_dir: &Path) -> Result<()> {
    let path = config_dir.join(FILE_NAME);
    let table = toml_value::read(&path)?;

    let baud = toml_value::whole(&table, BAUD_KEY, &path)?;
    if RESET_BAUDS.contains(&baud) {
        bail!(
            "{}: {BAUD_KEY} is {baud}: opening the port at that rate resets the board",
            path.display()
        );
    }

    let source = out_dir.join(SOURCE_FILE_NAME);
    fs::write(
        &source,
        format!(
            "/// The USB serial line's speed, bits per second: `{BAUD_KEY}`.\n\
             pub const USB_BAUD: u32 = {baud};\n"
        ),
    )
    .with_context(|| format!("writing {}", source.display()))
}
