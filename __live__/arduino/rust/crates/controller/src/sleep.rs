//! Sleeping until something happens, instead of asking over and over.

/// Sleeps in `WFI` until `look` finds what it is looking for, and returns it.
/// Interrupts are masked across the look and the sleep, so an interrupt that
/// comes between them can't be missed: a pending interrupt still ends `WFI`
/// while they are masked, and its handler runs once they are unmasked. What
/// the board wakes for is still there after the handler has run (a byte in
/// the UART's receive ring), so the next look finds it.
pub(crate) fn until<T>(mut look: impl FnMut() -> Option<T>) -> T {
    loop {
        cortex_m::interrupt::disable();
        let seen = look();
        if seen.is_none() {
            cortex_m::asm::wfi();
        }
        // Safety: nothing calls this inside a critical section, and the only
        // handlers linked are the UART's, which keep to their own state.
        unsafe { cortex_m::interrupt::enable() };
        if let Some(seen) = seen {
            return seen;
        }
    }
}
