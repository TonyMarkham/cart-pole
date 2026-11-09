//! Standard base64 (RFC 4648, section 4), decoded: `calibrate_service.py`
//! sends the live view's frame as base64 text in its JSON, and the viewer
//! needs the JPEG's bytes. Written out here, about twenty lines, rather than
//! taken from a crate.

/// The 64 characters, in order: each one's position is its 6-bit value
/// (RFC 4648, Table 1).
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bits each base64 character carries, and bits in a byte.
const BITS_PER_CHARACTER: u32 = 6;
const BITS_PER_BYTE: u32 = 8;

/// The character that pads the text to a whole number of 4-character groups.
const PADDING: u8 = b'=';

/// The bytes `text` encodes, or None if it holds a character outside the
/// alphabet. Padding ends the data; whitespace is not allowed.
pub(crate) fn decode(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for &character in text.as_bytes() {
        if character == PADDING {
            break;
        }
        let value = ALPHABET.iter().position(|&known| known == character)?;
        buffer = (buffer << BITS_PER_CHARACTER) | u32::try_from(value).ok()?;
        bits += BITS_PER_CHARACTER;
        if bits >= BITS_PER_BYTE {
            bits -= BITS_PER_BYTE;
            // The top 8 of the bits waiting make one byte; the rest wait on.
            bytes.push(u8::try_from(buffer >> bits).ok()?);
            buffer &= !(u32::MAX << bits);
        }
    }
    Some(bytes)
}
