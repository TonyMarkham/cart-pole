/// The DM542's direction input, by the direction it moves the cart: a
/// positive command's direction or a negative one's. Which pin level is
/// which is not decided yet (synthetic-arduino-plan.md, *Not decided by
/// the diagram*), so the level is not named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Positive,
    Negative,
}
