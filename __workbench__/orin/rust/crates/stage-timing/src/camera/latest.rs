use crate::camera::{frame::Frame, latest_state::LatestState};

use anyhow::{Result, anyhow, bail};
use std::{
    sync::{Condvar, Mutex},
    time::{Duration, Instant},
};

/// The newest frame, shared between the reader thread and the loop (the
/// brain's `camera/latest.rs`). The reader replaces it with every frame the
/// camera makes, so the pipe never fills; the loop takes only the frame it
/// asks for, and frames nobody asked for are dropped here.
#[derive(Debug, Default)]
pub(crate) struct Latest {
    state: Mutex<LatestState>,
    changed: Condvar,
}

impl Latest {
    /// The reader's new frame.
    pub(crate) fn put(&self, frame: Frame) {
        if let Ok(mut state) = self.state.lock() {
            state.seq = frame.seq;
            state.frame = Some(frame);
        }
        self.changed.notify_all();
    }

    /// The reader has stopped, and this is why. Whoever waits is woken.
    pub(crate) fn end(&self, reason: String) {
        if let Ok(mut state) = self.state.lock() {
            state.ended = Some(reason);
        }
        self.changed.notify_all();
    }

    /// How many frames have been read so far.
    pub(crate) fn seq(&self) -> u64 {
        self.state.lock().map(|state| state.seq).unwrap_or(0)
    }

    /// The first frame numbered after `seq`, within `timeout`. Fails when the
    /// camera has stopped, or makes no frame in time.
    pub(crate) fn next_after(&self, seq: u64, timeout: Duration) -> Result<Frame> {
        let deadline = Instant::now() + timeout;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("the frame lock is poisoned"))?;
        loop {
            if let Some(reason) = &state.ended {
                bail!("the camera stopped: {reason}");
            }
            if let Some(frame) = state.frame.as_ref().filter(|frame| frame.seq > seq) {
                return Ok(frame.clone());
            }
            let now = Instant::now();
            if now >= deadline {
                bail!(
                    "no frame within {} ms: the camera has stopped sending",
                    timeout.as_millis()
                );
            }
            let (next, _) = self
                .changed
                .wait_timeout(state, deadline - now)
                .map_err(|_| anyhow!("the frame lock is poisoned"))?;
            state = next;
        }
    }
}
