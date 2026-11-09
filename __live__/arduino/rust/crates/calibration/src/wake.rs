/// What a move found when it woke, besides a `Stop` datagram: each is
/// something the hardware latched, read once and cleared by the look.
#[derive(Clone, Copy)]
pub struct Wake {
    /// A 2 ms tick of the step generator has passed since the last look.
    pub tick: bool,
    /// Each switch's edge latch, in `Switch::ALL` order: a press since the
    /// last look, even one released again before it.
    pub edges: [bool; 2],
    /// The count stop fired: the generator stopped in hardware after exactly
    /// the pulses it was armed with.
    pub counted: bool,
    /// The wake timer's wait ended: a pressed switch's `trip_us` is up.
    pub timer: bool,
}

impl Wake {
    /// Whether the move has anything to look at. False when another
    /// interrupt woke the board, such as a byte from the WiFi chip.
    pub fn any(&self) -> bool {
        self.tick || self.edges.contains(&true) || self.counted || self.timer
    }
}
