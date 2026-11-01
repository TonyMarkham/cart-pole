use crate::constants::TEXT_MAX;

/// Short text from an AT reply: an address, a port or a version.
pub struct Text {
    bytes: [u8; TEXT_MAX],
    len: usize,
}

impl Text {
    pub fn new(text: &[u8]) -> Option<Self> {
        if text.is_empty() || text.len() > TEXT_MAX {
            return None;
        }
        let mut bytes = [0; TEXT_MAX];
        bytes[..text.len()].copy_from_slice(text);
        Some(Self {
            bytes,
            len: text.len(),
        })
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("?")
    }
}
