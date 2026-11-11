/// The rail's two limit switches as one tick reads them: `true` is closed,
/// tripped. In the loop they are the physical world's, so the orchestrator,
/// which stands in for it, works them out from MuJoCo's strike faces after
/// each timestep (__documentation__/synthetic-loop-dia.md, *The rail's ends*) and gives them
/// to the tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LimitSwitches {
    pub left: bool,
    pub right: bool,
}

impl LimitSwitches {
    /// Both open: the cart is clear of both ends.
    pub const OPEN: LimitSwitches = LimitSwitches {
        left: false,
        right: false,
    };

    /// Either switch closed, which is a FAULT (synthetic-arduino-plan.md, A8).
    pub const fn tripped(&self) -> bool {
        self.left || self.right
    }
}
