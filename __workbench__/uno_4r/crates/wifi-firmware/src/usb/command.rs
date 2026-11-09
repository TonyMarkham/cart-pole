use crate::constants::{
    BITS_PER_HEX_DIGIT, FIELD_SEPARATOR, HEX_RADIX, LINE_STATUS, SYNC_LINE_LEN, SYNC_PREFIX,
    U32_HEX_DIGITS,
};

/// A line from the PC's wifi-host that the board acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// `STATUS`: whether the board is running, and its address.
    Status,
    /// `SYNC <n> 00000000 00000000`: a clock sync, numbered `n`.
    Sync(u32),
}

impl Command {
    /// The command `line` is, or None for a line the board doesn't know.
    pub(crate) fn parse(line: &[u8]) -> Option<Self> {
        if line == LINE_STATUS {
            return Some(Self::Status);
        }
        let fields = line.strip_prefix(SYNC_PREFIX)?;
        if line.len() != SYNC_LINE_LEN {
            return None;
        }
        let (number, rest) = fields.split_at_checked(U32_HEX_DIGITS)?;
        if rest.first() != Some(&FIELD_SEPARATOR) {
            return None;
        }
        Some(Self::Sync(hex_u32(number)?))
    }
}

/// The number in `digits`, hex, upper or lower case.
fn hex_u32(digits: &[u8]) -> Option<u32> {
    digits.iter().try_fold(0u32, |value, &digit| {
        let nibble = char::from(digit).to_digit(HEX_RADIX)?;
        Some(value << BITS_PER_HEX_DIGIT | nibble)
    })
}
