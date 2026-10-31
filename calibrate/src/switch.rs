/// A limit switch, by the pin it is wired to (control-design.md: D2 and
/// D3, both IRQ pins). Homing treats either as an end of the rail; the
/// report names the pin at each end, so a wiring fault shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    D2,
    D3,
}

impl Switch {
    pub const ALL: [Switch; 2] = [Switch::D2, Switch::D3];

    /// The switch's place in `ALL`.
    pub const fn index(self) -> usize {
        match self {
            Switch::D2 => 0,
            Switch::D3 => 1,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Switch::D2 => "D2",
            Switch::D3 => "D3",
        }
    }
}
