/// Which way the cart goes in the drawing, with the motor on the left. None for
/// a centring whose starting end the page does not know.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Heading {
    Left,
    Right,
}

impl Heading {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Heading::Left => "left",
            Heading::Right => "right",
        }
    }
}
