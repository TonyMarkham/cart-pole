use serde::Deserialize;

/// The part of the webserver's `/arduino/settings` this page uses.
#[derive(Deserialize)]
pub(crate) struct Settings {
    pub(crate) address: String,
    pub(crate) poll_ms: u32,
}
