use crate::text::Text;

/// What the join found, for the serial port.
pub struct Joined {
    pub address: Text,
    pub bridge_version: Option<Text>,
}
