use crate::cart::seek_end::SeekEnd;

/// What woke the board during a seek with something to act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SeekWake {
    /// The seek is over.
    Ended(SeekEnd),
    /// The cart has moved `release_pulses` off the motor's switch it started
    /// on, so that switch should read released now.
    LeftSwitch,
}
