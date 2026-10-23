/// Width, height, and channel count read from a JPEG's start-of-frame
/// header — enough to check the frame contract (728×544) without
/// decoding pixels. The player's greyscale frames are encoded as three
/// equal channels, so greyscale shows as 3 channels here; whether they
/// are actually grey is checked by eye on the saved files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegInfo {
    pub width: u16,
    pub height: u16,
    pub channels: u8,
}

impl JpegInfo {
    /// Walks the marker segments after SOI (FF D8) to the first SOF
    /// (FF C0..CF, excluding C4 DHT, C8 JPG, CC DAC), whose payload is
    /// precision(1) height(2) width(2) channels(1), big-endian. `None`
    /// when the bytes are not a JPEG or carry no SOF before the image
    /// data starts.
    pub fn read(bytes: &[u8]) -> Option<JpegInfo> {
        if bytes.get(0..2)? != [0xFF, 0xD8] {
            return None;
        }
        let mut at = 2;
        loop {
            if *bytes.get(at)? != 0xFF {
                return None;
            }
            let marker = *bytes.get(at + 1)?;
            // Fill bytes (FF FF) pad between segments.
            if marker == 0xFF {
                at += 1;
                continue;
            }
            // Standalone markers carry no length field.
            if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
                at += 2;
                continue;
            }
            // Start of scan: compressed data follows, no SOF was seen.
            if marker == 0xDA {
                return None;
            }
            let length = usize::from(u16::from_be_bytes([*bytes.get(at + 2)?, *bytes.get(at + 3)?]));
            let is_sof = (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker);
            if is_sof {
                let payload = bytes.get(at + 4..at + 2 + length)?;
                return Some(JpegInfo {
                    height: u16::from_be_bytes([*payload.get(1)?, *payload.get(2)?]),
                    width: u16::from_be_bytes([*payload.get(3)?, *payload.get(4)?]),
                    channels: *payload.get(5)?,
                });
            }
            at += 2 + length;
        }
    }
}
