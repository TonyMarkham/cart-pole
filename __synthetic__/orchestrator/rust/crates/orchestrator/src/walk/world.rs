use anyhow::Result;
use arduino::LimitSwitches;

/// The world the walk moves through, one timestep at a time: MuJoCo and the
/// synthetic Arduino, which the orchestrator holds (`Simulation`), or a
/// stand-in in the tests.
pub trait World {
    /// MuJoCo moves on one timestep (15–16); returns the two limit switches,
    /// worked out from the strike faces there.
    fn move_on(&mut self) -> Result<LimitSwitches>;

    /// The synthetic Arduino's tick (19–21): `receive(landed)` first when a
    /// command landed on this timestep, then the tick with `switches`, and
    /// ctrl set from its signals. Returns whether it is in FAULT.
    fn tick(&mut self, landed: Option<i32>, switches: LimitSwitches) -> Result<bool>;
}
