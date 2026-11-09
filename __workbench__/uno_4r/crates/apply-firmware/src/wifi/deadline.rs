use core::time::Duration;
use unor4_hal::wake::{WakeError, WakeTimer};

/// A time limit on the wake timer, of any length. One wait on the timer lasts
/// at most `WakeTimer::longest` (1.398 s on GPT167), so a longer limit is timed
/// as a run of waits, each started when the one before ends. The timer keeps
/// running while the board handles a byte, so the limit counts all the time
/// since `start`, not only the time spent asleep.
pub(crate) struct Deadline {
    /// What is left after the wait the timer is running now.
    after: Duration,
}

impl Deadline {
    /// Starts timing `limit`, replacing any wait the timer is running.
    pub(crate) fn start(wake: &mut WakeTimer, limit: Duration) -> Result<Self, WakeError> {
        let first = limit.min(wake.longest());
        wake.start(first)?;
        Ok(Self {
            after: limit - first,
        })
    }

    /// Whether the limit has passed: the timer's wait has ended and nothing is
    /// left. A wait that ended with time still left starts the next one. Works
    /// with interrupts masked: `expired` reads the timer's own flag
    /// (unor4-hal `wake/mod.rs`, *Reading the end*).
    pub(crate) fn passed(&mut self, wake: &mut WakeTimer) -> Result<bool, WakeError> {
        if !wake.expired() {
            return Ok(false);
        }
        if self.after.is_zero() {
            return Ok(true);
        }
        let next = self.after.min(wake.longest());
        wake.start(next)?;
        self.after -= next;
        Ok(false)
    }
}
