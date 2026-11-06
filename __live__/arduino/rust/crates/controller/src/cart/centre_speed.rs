use crate::config::calibration::{CENTRE_SPEED_MAX, SEEK_SPEED};

/// The speed a move to the centre asks for, as the Orin sent it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CentreSpeed {
    /// `MoveToCentre`: no speed, so `seek_speed`.
    Default,
    /// `MoveToCentre <n>`: a whole number, not yet checked against the range.
    Asked(u32),
    /// `MoveToCentre <text>` where the text isn't a whole number.
    Bad,
}

impl CentreSpeed {
    /// The speed in `digits`, the text after `MoveToCentre `: `Asked` for a
    /// whole number that fits a u32, `Bad` for anything else, an empty text
    /// included.
    pub(crate) fn parse(digits: &[u8]) -> Self {
        if digits.is_empty() {
            return Self::Bad;
        }
        let mut speed: u32 = 0;
        for &digit in digits {
            if !digit.is_ascii_digit() {
                return Self::Bad;
            }
            let Some(next) = speed
                .checked_mul(10)
                .and_then(|speed| speed.checked_add(u32::from(digit - b'0')))
            else {
                return Self::Bad;
            };
            speed = next;
        }
        Self::Asked(speed)
    }

    /// The speed to run at, pulses per second, or None for one outside 1 to
    /// `centre_speed_max`: the firmware alone decides what a good speed is
    /// (the webserver's `calibrate.rs`, `CentreQuery`).
    pub(crate) fn speed(self) -> Option<i32> {
        match self {
            Self::Default => Some(SEEK_SPEED),
            Self::Asked(speed) => i32::try_from(speed)
                .ok()
                .filter(|speed| (1..=CENTRE_SPEED_MAX).contains(speed)),
            Self::Bad => None,
        }
    }
}
