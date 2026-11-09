//! Sleeping until something happens, instead of asking over and over: the
//! controller's `sleep.rs`, and a marked form for the run loop.

use crate::pins::marker::Marker;

/// Sleeps in `WFI` until `look` finds what it is looking for, and returns it.
/// Interrupts are masked across the look and the sleep, so an interrupt that
/// comes between them can't be missed: a pending interrupt still ends `WFI`
/// while they are masked, and its handler runs once they are unmasked. The
/// controller's `sleep::until`, unchanged; the join uses it.
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

/// As `until`, with D3 marking the processor's state for the R3: low from
/// just before `WFI`, high again just after it ends. So each high stretch is
/// one time the processor was awake, from the wake to the next sleep, and it
/// includes the interrupt handlers, which run once interrupts are unmasked.
/// The marks cost two register writes per wake (unor4-hal `gpio/pin.rs`,
/// `set_high`, `set_low`: one atomic port write each).
pub(crate) fn until_marked<T>(marker: &mut Marker, mut look: impl FnMut() -> Option<T>) -> T {
    loop {
        cortex_m::interrupt::disable();
        let seen = look();
        if seen.is_none() {
            marker.asleep();
            cortex_m::asm::wfi();
            marker.awake();
        }
        // Safety: as in `until`.
        unsafe { cortex_m::interrupt::enable() };
        if let Some(seen) = seen {
            return seen;
        }
    }
}
