use core::fmt::{self, Write};

/// The level of every pin the drive uses, `true` = high. Inputs are read
/// from the pin; outputs are the level the firmware drives (the port's
/// output latch).
#[derive(Clone, Copy, Debug)]
pub struct PinStates {
    /// D2: one switch's NC line.
    pub d2: bool,
    /// D3: the other switch's NC line.
    pub d3: bool,
    /// D5: DIR, driven; low for Minus.
    pub dir: bool,
    /// D4: STEP, driven; rests high, low during a pulse.
    pub step: bool,
    /// D6: ENABLE, driven; low disables the driver.
    pub enable: bool,
    /// D8: unconnected (the DRV8825's FAULT input; the DM542 has none).
    pub fault: bool,
}

impl PinStates {
    /// One line per pin: the pin, its role, its level (1 = high), and
    /// what that level means on this rig (control-design.md).
    pub fn report(&self, out: &mut impl Write) -> fmt::Result {
        let switch = |high| if high { "pressed, or its wire is open" } else { "released" };
        writeln!(out, "D2 switch       {}  {}", u8::from(self.d2), switch(self.d2))?;
        writeln!(out, "D3 switch       {}  {}", u8::from(self.d3), switch(self.d3))?;
        writeln!(out, "D4 STEP         {}", u8::from(self.step))?;
        writeln!(
            out,
            "D5 DIR          {}  {}",
            u8::from(self.dir),
            if self.dir { "plus" } else { "minus, the way the homing's first seek runs" }
        )?;
        writeln!(
            out,
            "D6 ENABLE       {}  {}",
            u8::from(self.enable),
            if self.enable { "driver enabled" } else { "driver disabled, motor free" }
        )?;
        writeln!(
            out,
            "D8 FAULT        {}  {}",
            u8::from(self.fault),
            if self.fault { "unconnected" } else { "low: something is wired to D8" }
        )
    }
}
