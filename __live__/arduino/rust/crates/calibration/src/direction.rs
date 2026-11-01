/// A direction of travel along the rail, by the level on D5 (DIR). Which
/// physical end each leads to depends on the motor's wiring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    High,
    Low,
}
