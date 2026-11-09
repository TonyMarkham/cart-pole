use crate::constants::{
    GDP_HEADER_LENGTH, GDP_LENGTH_OFFSET, GDP_MAJOR_VERSION, GDP_PAYLOAD_BUFFER,
    GDP_TIMESTAMP_OFFSET, GDP_TYPE_OFFSET,
};

/// The three fields the brain reads from a GStreamer Data Protocol header: what
/// the payload is, how long it is, and the buffer's timestamp. `gdppay` writes
/// one such header before every packet it sends down the pipe, frames and
/// GStreamer's own messages alike, so a packet whose type is not a buffer is
/// skipped by its length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GdpHeader {
    pub(crate) payload_type: u16,
    pub(crate) payload_length: u32,
    /// The buffer's timestamp, ns; GStreamer's "none" is every bit set.
    pub(crate) timestamp_ns: u64,
}

impl GdpHeader {
    /// The header in `bytes`, or why it is not one: a protocol version other
    /// than 1 means the pipe does not carry GDP at all.
    pub(crate) fn parse(bytes: &[u8; GDP_HEADER_LENGTH]) -> Result<GdpHeader, String> {
        let major = bytes[0];
        if major != GDP_MAJOR_VERSION {
            return Err(format!(
                "the camera's pipe is not GDP version 1 (first byte {major}, expected \
                 {GDP_MAJOR_VERSION}): is gdppay in the pipeline?"
            ));
        }
        Ok(GdpHeader {
            payload_type: u16::from_be_bytes(
                bytes[GDP_TYPE_OFFSET..GDP_TYPE_OFFSET + 2]
                    .try_into()
                    .map_err(|_| "the header's type field is short".to_string())?,
            ),
            payload_length: u32::from_be_bytes(
                bytes[GDP_LENGTH_OFFSET..GDP_LENGTH_OFFSET + 4]
                    .try_into()
                    .map_err(|_| "the header's length field is short".to_string())?,
            ),
            timestamp_ns: u64::from_be_bytes(
                bytes[GDP_TIMESTAMP_OFFSET..GDP_TIMESTAMP_OFFSET + 8]
                    .try_into()
                    .map_err(|_| "the header's timestamp field is short".to_string())?,
            ),
        })
    }

    /// Whether the payload is a frame.
    pub(crate) fn is_buffer(self) -> bool {
        self.payload_type == GDP_PAYLOAD_BUFFER
    }
}
