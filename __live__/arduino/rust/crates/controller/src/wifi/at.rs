//! One exchange with the WiFi chip at a time: a command and its reply, the
//! drain before it, and a pause between tries. Every wait sleeps until a byte
//! arrives or the wake timer ends it (`next_byte`, `Deadline`).

use crate::{
    config::wifi_timing::DRAIN_QUIET_MS,
    sleep,
    wifi::{AT, deadline::Deadline, next_byte, reply::Reply},
};

use core::{fmt, time::Duration};
use unor4_hal::wake::{WakeError, WakeTimer};

/// Before push mode: throws away stale bytes, then sends `command` and waits
/// up to `timeout_ms` for its reply. None if no reply ended in time.
pub(crate) fn command(
    wake: &mut WakeTimer,
    command: &[u8],
    timeout_ms: u32,
) -> Result<Option<Reply>, WakeError> {
    drain(wake)?;
    AT.write_all(command);
    reply(wake, timeout_ms)
}

/// Collects a reply until its last line, or None once `timeout_ms` has passed.
fn reply(wake: &mut WakeTimer, timeout_ms: u32) -> Result<Option<Reply>, WakeError> {
    let mut deadline = Deadline::start(wake, millis(timeout_ms))?;
    let mut reply = Reply::new();
    while let Some(byte) = next_byte(wake, &mut deadline)? {
        if reply.push(byte) {
            return Ok(Some(reply));
        }
    }
    Ok(None)
}

/// Throws away bytes until the link has been quiet for `drain_quiet_ms`: the
/// wait starts again at every byte.
pub(crate) fn drain(wake: &mut WakeTimer) -> Result<(), WakeError> {
    loop {
        let mut quiet = Deadline::start(wake, millis(DRAIN_QUIET_MS))?;
        if next_byte(wake, &mut quiet)?.is_none() {
            return Ok(());
        }
    }
}

/// Sleeps for `ms`, as the join's pauses between tries. A byte arriving
/// meanwhile wakes the board and stays in the ring.
pub(crate) fn pause(wake: &mut WakeTimer, ms: u32) -> Result<(), WakeError> {
    let mut deadline = Deadline::start(wake, millis(ms))?;
    sleep::until(|| match deadline.passed(wake) {
        Ok(true) => Some(Ok(())),
        Ok(false) => None,
        Err(error) => Some(Err(error)),
    })
}

/// `ms` as a `Duration`: every time in `config/wifi_timing.toml` is in ms.
pub(crate) fn millis(ms: u32) -> Duration {
    Duration::from_millis(u64::from(ms))
}

/// As `command`, for a command written from `arguments`, such as one naming
/// the socket.
pub(crate) fn command_fmt(
    wake: &mut WakeTimer,
    arguments: fmt::Arguments,
    timeout_ms: u32,
) -> Result<Option<Reply>, WakeError> {
    drain(wake)?;
    // Writing to the link never fails (`fmt::Write for &UartIrq`).
    let _ = fmt::Write::write_fmt(&mut &AT, arguments);
    reply(wake, timeout_ms)
}
