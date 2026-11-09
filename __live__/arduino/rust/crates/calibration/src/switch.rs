/// A limit switch, by the pin it is wired to (control-design.md: D2 and D3).
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

    /// Its name in the board's answers, as the `pins` answer writes it.
    pub const fn name(self) -> &'static str {
        match self {
            Switch::D2 => "d2",
            Switch::D3 => "d3",
        }
    }
}
