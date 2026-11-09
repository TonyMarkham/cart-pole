use serde::Deserialize;

/// `[send]`: where the command datagrams go, explained in
/// `stage-timing.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SendSettings {
    pub(crate) address: String,
    pub(crate) port: u16,
}
