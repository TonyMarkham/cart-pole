use crate::config::wifi_timing::{STATUS_PAUSE_MS, STATUS_TRIES};

use core::fmt;
use unor4_hal::wake::WakeError;

/// Why the join failed, in words for `OK STATUS stopped wifi: …`. The words
/// are calibration's (its `Wifi::join`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JoinError {
    /// The chip never answered the first `AT`.
    NoAnswer,
    /// The chip refused station mode.
    NoStationMode,
    /// The chip refused to start the join.
    JoinRefused,
    /// The router's network wasn't found.
    NetworkNotFound,
    /// The network was found, but joining it failed.
    ConnectFailed,
    /// The join didn't finish in `status_tries` × `status_pause_ms`.
    JoinTimedOut,
    /// The join finished, but the router gave no address.
    NoAddress,
    /// The chip refused to open the UDP socket.
    NoSocket,
    /// The chip has no push mode: it isn't running the interrupt bridge.
    NoPushMode,
    /// The wake timer refused a wait.
    WakeTimer(WakeError),
}

impl From<WakeError> for JoinError {
    fn from(error: WakeError) -> Self {
        Self::WakeTimer(error)
    }
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoAnswer => f.write_str("the ESP32-S3 bridge never answered AT"),
            Self::NoStationMode => f.write_str("the bridge refused station mode"),
            Self::JoinRefused => f.write_str("the bridge refused the join"),
            Self::NetworkNotFound => f.write_str(
                "network not found (out of range, or 5 GHz only: the radio is 2.4 GHz)",
            ),
            Self::ConnectFailed => f.write_str("connect failed: usually a wrong password"),
            Self::JoinTimedOut => write!(
                f,
                "the join did not finish within {} ms",
                STATUS_TRIES * STATUS_PAUSE_MS
            ),
            Self::NoAddress => f.write_str("joined, but the router gave no address"),
            Self::NoSocket => f.write_str("the bridge refused to open the UDP socket"),
            Self::NoPushMode => f.write_str(
                "this bridge has no push mode: flash the interrupt bridge (its docs/flash-interrupt-procedure.md)",
            ),
            Self::WakeTimer(error) => write!(f, "wake timer: {error:?}"),
        }
    }
}
