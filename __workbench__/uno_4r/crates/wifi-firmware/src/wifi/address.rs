use crate::constants::ADDRESS_MAX;

/// The address the router gave the board, as the join read it from
/// `+IPSTA: `, for the `STATUS` answer: the PC sends its datagrams there.
#[derive(Clone, Copy)]
pub(crate) struct Address {
    bytes: [u8; ADDRESS_MAX],
    len: usize,
}

impl Address {
    /// The address in `text`, or None for one longer than an IPv4 address.
    pub(crate) fn new(text: &[u8]) -> Option<Self> {
        if text.len() > ADDRESS_MAX {
            return None;
        }
        let mut bytes = [0; ADDRESS_MAX];
        bytes[..text.len()].copy_from_slice(text);
        Some(Self {
            bytes,
            len: text.len(),
        })
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
