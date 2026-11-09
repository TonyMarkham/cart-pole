use serde::Deserialize;

/// `[arduino]`: where the rig's Arduino listens for the command datagrams. The
/// table's other keys are the webserver's waits and repeats, so unknown keys
/// are allowed here; `settings.toml` explains each value where it is set.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ArduinoSettings {
    pub(crate) address: String,
    pub(crate) port: u16,
}
