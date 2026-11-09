//! `config/wifi.toml` and the credentials file it names: the settings the
//! board joins the network with.

use crate::toml_value;

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

/// The settings file in `config/`, and the file written into `OUT_DIR`, which
/// `src/config/wifi.rs` includes by the same name.
const FILE_NAME: &str = "wifi.toml";
const SOURCE_FILE_NAME: &str = "wifi.rs";

/// The keys.
const CREDENTIALS_KEY: &str = "credentials";
const AT_BAUD_KEY: &str = "at_baud";
const HOSTNAME_KEY: &str = "hostname";
const UDP_PORT_KEY: &str = "udp_port";

/// The credentials file's keys, and its template, named in the message when
/// the file is missing.
const SSID_KEY: &str = "ssid";
const PASSWORD_KEY: &str = "password";
const CREDENTIALS_EXAMPLE_FILE_NAME: &str = "wifi_credentials.example.toml";

/// The longest hostname the board takes: 31 characters.
const HOSTNAME_MAX: usize = 31;

/// WPA2's limits: an SSID of 1 to 32 bytes, a passphrase of 8 to 63
/// characters, or none for an open network.
const SSID_MAX: usize = 32;
const PASSWORD_MIN: usize = 8;
const PASSWORD_MAX: usize = 63;

/// Reads `config/wifi.toml` and its credentials file, and writes their
/// constants into `out_dir`.
pub fn generate(config_dir: &Path, out_dir: &Path) -> Result<()> {
    let path = config_dir.join(FILE_NAME);
    let table = toml_value::read(&path)?;

    let at_baud = toml_value::whole(&table, AT_BAUD_KEY, &path)?;

    let udp_port = toml_value::whole(&table, UDP_PORT_KEY, &path)?;
    let Ok(udp_port) = u16::try_from(udp_port) else {
        bail!(
            "{}: {UDP_PORT_KEY} is {udp_port}; a port is at most {}",
            path.display(),
            u16::MAX
        );
    };

    let hostname = toml_value::text(&table, HOSTNAME_KEY, &path)?;
    if hostname.is_empty()
        || hostname.len() > HOSTNAME_MAX
        || !hostname
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!(
            "{}: {HOSTNAME_KEY} `{hostname}`: letters, digits and hyphens, 1 to {HOSTNAME_MAX} of them",
            path.display()
        );
    }

    let credentials_path = config_dir.join(toml_value::text(&table, CREDENTIALS_KEY, &path)?);
    if !credentials_path.exists() {
        bail!(
            "{} is missing: copy {CREDENTIALS_EXAMPLE_FILE_NAME} to that name and fill in the network",
            credentials_path.display()
        );
    }
    let credentials = toml_value::read(&credentials_path)?;

    let ssid = toml_value::text(&credentials, SSID_KEY, &credentials_path)?;
    if ssid.is_empty() || ssid.len() > SSID_MAX || !toml_value::printable(ssid) {
        bail!(
            "{}: {SSID_KEY} must be 1 to {SSID_MAX} printable ASCII characters",
            credentials_path.display()
        );
    }

    let password = toml_value::text(&credentials, PASSWORD_KEY, &credentials_path)?;
    if !(password.is_empty() || (PASSWORD_MIN..=PASSWORD_MAX).contains(&password.len()))
        || !toml_value::printable(password)
    {
        bail!(
            "{}: {PASSWORD_KEY} must be empty or {PASSWORD_MIN} to {PASSWORD_MAX} printable ASCII characters",
            credentials_path.display()
        );
    }

    let at_hostname = format!("AT+HOSTNAME=\"{hostname}\"\r\n");
    let at_beginsta = format!(
        "AT+BEGINSTA=\"{}\",\"{}\"\r\n",
        at_quoted(ssid),
        at_quoted(password)
    );
    let at_udpbegin = format!("AT+UDPBEGIN={udp_port}\r\n");

    // `{:?}` writes each string as a Rust literal, so a quote or a backslash
    // in it can't break the generated file.
    let source = out_dir.join(SOURCE_FILE_NAME);
    fs::write(
        &source,
        format!(
            "/// The link to the WiFi chip's speed, bits per second: `{AT_BAUD_KEY}`.\n\
             pub const AT_BAUD: u32 = {at_baud};\n\
             /// Stores the name the router lists: `{HOSTNAME_KEY}`.\n\
             pub const AT_HOSTNAME: &[u8] = {at_hostname:?}.as_bytes();\n\
             /// Joins the network in the credentials file.\n\
             pub const AT_BEGINSTA: &[u8] = {at_beginsta:?}.as_bytes();\n\
             /// Opens the UDP socket on `{UDP_PORT_KEY}`.\n\
             pub const AT_UDPBEGIN: &[u8] = {at_udpbegin:?}.as_bytes();\n\
             /// The port itself, for the `STATUS` answer: `{UDP_PORT_KEY}`.\n\
             pub const UDP_PORT: u16 = {udp_port};\n"
        ),
    )
    .with_context(|| format!("writing {}", source.display()))
}

/// `text` inside an AT command's quotes: each `"` and `\` gets a `\` before it.
/// The bridge's parser splits a command at unquoted commas and takes `\` as an
/// escape, so a password holding either would otherwise break `AT+BEGINSTA`.
fn at_quoted(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '"' || character == '\\' {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted
}
