use ra4m1::{GPT320, MSTP, SYSTEM};

/// `PRCR`'s key with PRC1 set, which unlocks the module-stop registers, and
/// its relock (RA4M1 User's Manual, 12.2.1 PRCR; the values unor4-hal's
/// `pwm/gpt_channel.rs` writes, `PRCR_UNLOCK_PRC1` and `PRCR_LOCK`).
const PRCR_UNLOCK_PRC1: u16 = 0xA502;
const PRCR_LOCK: u16 = 0xA500;

/// The board's clock for this test: GPT320, a 32-bit timer counting PCLKD,
/// 48 MHz (unor4-hal `clock.rs`, HOCO), so one count is 20.8 ns and the count
/// wraps every 2³² / 48 MHz = 89.5 s. The PC widens it to 64 bits: its
/// stamps are milliseconds apart, far inside one wrap.
///
/// Why this timer: the processor's own cycle counter stops while it sleeps in
/// `WFI`, and the board sleeps between every byte. A GPT keeps counting
/// through sleep, as the step generator's do. GPT320 is free here because
/// this firmware builds no step generator (the controller's uses GPT320 for
/// STEP and GPT321 for its count).
///
/// The true rate is the HOCO's, which differs from 48 MHz by its tolerance:
/// the PC measures it, from the clock syncs at both ends of the run.
pub(crate) struct Clock {
    timer: GPT320,
}

impl Clock {
    /// Takes GPT320 out of module stop, and starts it counting up from 0
    /// through its whole 32 bits. The register writes are the step
    /// generator's counter set-up (unor4-hal `step/step_generator.rs`), with
    /// no output, no buffer and no start sources: saw-wave mode, PCLKD/1, up,
    /// period 2³² − 1, started by software.
    pub(crate) fn new(timer: GPT320) -> Self {
        // Safety: the HAL gave GPT320 to this firmware, and only these two
        // registers of SYSTEM and MSTP are touched, unlocked and relocked
        // around the one write, as the HAL's own release does.
        let system = unsafe { &*SYSTEM::PTR };
        let mstp = unsafe { &*MSTP::PTR };
        system.prcr.write(|w| unsafe { w.bits(PRCR_UNLOCK_PRC1) });
        mstp.mstpcrd.modify(|_, w| w.mstpd5()._0());
        system.prcr.write(|w| unsafe { w.bits(PRCR_LOCK) });

        timer
            .gtcr
            .write(|w| unsafe { w.md()._000().tpcs().bits(0).cst().clear_bit() });
        timer.gtuddtyc.write(|w| w.ud().set_bit());
        timer.gtber.write(|w| unsafe { w.bits(0) });
        timer.gtssr.write(|w| unsafe { w.bits(0) });
        timer.gtpsr.write(|w| unsafe { w.bits(0) });
        timer.gtcsr.write(|w| unsafe { w.bits(0) });
        timer.gtupsr.write(|w| unsafe { w.bits(0) });
        timer.gtdnsr.write(|w| unsafe { w.bits(0) });
        timer.gtpr.write(|w| unsafe { w.bits(u32::MAX) });
        timer.gtcnt.write(|w| unsafe { w.bits(0) });
        timer.gtcr.modify(|_, w| w.cst().set_bit());
        Self { timer }
    }

    /// The count now.
    pub(crate) fn now(&self) -> u32 {
        self.timer.gtcnt.read().bits()
    }
}
