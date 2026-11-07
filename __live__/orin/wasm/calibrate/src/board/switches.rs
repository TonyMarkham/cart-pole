use serde::Deserialize;

/// The part of a pin reading this page uses, 1 = high. A limit switch's contact
/// is closed until pressed, so a pressed switch or a broken wire reads high.
/// Which pin is at which end must match `[motor]` and `[idler]` in the firmware's
/// `calibration.toml`: the motor's end is D2 and the idler's D3.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) struct Switches {
    pub(crate) d2: u8,
    pub(crate) d3: u8,
}

impl Switches {
    pub(crate) fn motor_pressed(self) -> bool {
        self.d2 == 1
    }

    pub(crate) fn idler_pressed(self) -> bool {
        self.d3 == 1
    }
}
