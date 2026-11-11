use unor4_hal::gpio::{
    Pull, SwitchPinBuilder, SwitchSetup,
    contact::Nc,
    pin_map::{self, D3},
};

/// The limit switch at the idler's end, on D3. Its NC (normally closed)
/// contact holds the pin to GND until the cart presses it, so a press reads
/// high, and so does a broken wire. The board's own pull-up is the line's
/// only one (__documentation__/control-design.md). `with_edge` latches each press in hardware,
/// even one released before the processor looks, and `with_wake` makes the
/// press end a `WFI`, so the board sleeps until the switch is pressed instead
/// of reading it again and again.
pub(crate) const IDLER_LIMIT_SWITCH: SwitchSetup<D3, Nc> = SwitchPinBuilder::new()
    .with_pin(pin_map::d3)
    .with_contact(Nc)
    .with_pull(Pull::Internal)
    .with_edge()
    .with_wake()
    .build();
