use crate::constants::ADDRESS_MAX;

/// Who sent a datagram: the address and port the bridge reports for it
/// (`AT+UDPREMOTEIP`, `AT+UDPREMOTEPORT`). A move's answer goes to whoever
/// asked for the move, read before it starts: the `Stop` that can end it
/// comes from another socket (the webserver's `calibrate.rs`, `send_stops`),
/// and an answer sent there would leave the move's own wait to time out.
pub(crate) struct Asker {
    address: [u8; ADDRESS_MAX],
    len: usize,
    port: u16,
}

impl Asker {
    /// The asker at `address` and `port`, or None for an address longer than
    /// an IPv4 one.
    pub(crate) fn new(address: &str, port: u16) -> Option<Self> {
        let bytes = address.as_bytes();
        if bytes.len() > ADDRESS_MAX {
            return None;
        }
        let mut kept = [0; ADDRESS_MAX];
        kept[..bytes.len()].copy_from_slice(bytes);
        Some(Self {
            address: kept,
            len: bytes.len(),
            port,
        })
    }

    pub(crate) fn address(&self) -> &str {
        // Never fails: `new` kept a whole `&str`.
        core::str::from_utf8(&self.address[..self.len]).unwrap_or_default()
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }
}
