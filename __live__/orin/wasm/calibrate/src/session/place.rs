/// Where the drawing puts the cart. Only a place the page has a reason for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Place {
    Motor,
    Idler,
    Centre,
    /// Neither switch reads pressed and no centring has worked since one did.
    Unknown,
    /// Both switches read pressed, which one cart cannot do.
    BothPressed,
    Moving,
}

impl Place {
    /// The sentence under the drawing.
    pub(crate) fn sentence(self) -> &'static str {
        match self {
            Place::Motor => {
                "D2 reads pressed, so the cart should be at the motor end. A broken wire reads the same."
            }
            Place::Idler => {
                "D3 reads pressed, so the cart should be at the idler end. A broken wire reads the same."
            }
            Place::Centre => "The last centring worked, and neither switch has read pressed since.",
            Place::Unknown => {
                "Neither switch reads pressed, so the cart is somewhere between them. This page does not know where."
            }
            Place::BothPressed => {
                "Both switches read pressed, which one cart cannot do. Check the wiring, and take out any USB cable from the Pi."
            }
            Place::Moving => "The cart is moving. Its position is not known until it stops.",
        }
    }
}
