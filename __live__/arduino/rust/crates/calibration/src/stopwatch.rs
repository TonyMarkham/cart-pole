use crate::constants::{DEMCR_TRCENA, DWT_CTRL_CYCCNTENA};

use cortex_m::peripheral::{DCB, DWT};

/// Time since `start`, from the core's cycle counter (DWT CYCCNT), which
/// counts ICLK cycles and wraps every 2³² of them, 89 s at 48 MHz. `cycles`
/// adds each reading's step to a 64-bit total, so the time stays right as
/// long as it is read at least once per wrap; a seek reads it every pulse.
pub struct Stopwatch {
    last: u32,
    cycles: u64,
    cycles_per_ms: u32,
}

impl Stopwatch {
    /// Starts timing. `sysclk` is ICLK in Hz (`Clocks::sysclk`).
    pub fn start(sysclk: u32) -> Self {
        // The two writes only set enable bits; the counter is read-only
        // here otherwise, and nothing else in the program uses the DWT.
        unsafe {
            (*DCB::PTR).demcr.modify(|value| value | DEMCR_TRCENA);
            (*DWT::PTR).ctrl.modify(|value| value | DWT_CTRL_CYCCNTENA);
        }
        Self {
            last: DWT::cycle_count(),
            cycles: 0,
            cycles_per_ms: (sysclk / 1_000).max(1),
        }
    }

    /// ICLK cycles since `start`.
    pub fn cycles(&mut self) -> u64 {
        let now = DWT::cycle_count();
        self.cycles += u64::from(now.wrapping_sub(self.last));
        self.last = now;
        self.cycles
    }

    /// Whole ms since `start`.
    pub fn ms(&mut self) -> u64 {
        self.cycles() / u64::from(self.cycles_per_ms)
    }
}
