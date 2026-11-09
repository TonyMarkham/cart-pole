pub(crate) mod location;
pub(crate) mod result;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    answer::write_answer,
    constants::{
        STATUS_BAD_REQUEST, STATUS_CONFLICT, STATUS_INTERNAL_ERROR, STATUS_REQUEST_TIMEOUT,
    },
    error::location::ErrorLocation,
    settings::{BAD_SPEED_MESSAGE, TIMED_OUT_MESSAGE},
    switch::Switch,
};

use core::{
    fmt::{self, Write},
    panic::Location,
};
use unor4_hal::{step::StepError, wake::WakeError};

/// The pattern of the Orin webserver's `AppError`, with each message a
/// `&'static str`: the board has no allocator, so no `String`. Every way a move
/// can fail is here, with its message and, in `write_answer`, its status.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AppError {
    #[error("[Bridge Error]: {message}{location}")]
    Bridge {
        message: &'static str,
        location: ErrorLocation,
    },

    #[error("[Format Error]: {message}{location}")]
    Format {
        message: &'static str,
        location: ErrorLocation,
    },

    #[error("[Join Error]: {message}{location}")]
    Join {
        message: &'static str,
        location: ErrorLocation,
    },

    /// That end's switch already reads pressed. The cart did not move: going on
    /// would push it past a switch that cannot trip again. It does not count as
    /// finding that end, because a broken wire reads the same.
    #[error("[Already There Error]: {message}{location}")]
    AlreadyThere {
        switch: Switch,
        message: &'static str,
        location: ErrorLocation,
    },

    /// A centring whose speed was not a whole number from 1 to
    /// `centre_speed_max`. The cart did not move.
    #[error("[Bad Speed Error]: {message}{location}")]
    BadSpeed {
        message: &'static str,
        location: ErrorLocation,
    },

    /// A centring with no span yet, or with the cart not resting on a switch.
    /// The cart did not move.
    #[error("[Not Ready Error]: {message}{location}")]
    NotReady {
        message: &'static str,
        location: ErrorLocation,
    },

    /// Both switches read pressed, so no move can be told from a fault: an open
    /// wire reads pressed, and so do both switches with a USB cable in from the
    /// Pi (the manager's README). The cart did not move.
    #[error("[Refused Error]: {message}{location}")]
    Refused {
        message: &'static str,
        location: ErrorLocation,
    },

    /// The HAL's step generator refused to start the move, for the reason it
    /// gives. The cart did not move.
    #[error("[Stepper Error]: {message}: {error:?}{location}")]
    Stepper {
        error: StepError,
        message: &'static str,
        location: ErrorLocation,
    },

    /// A `Stop` datagram ended the move after this many pulses.
    #[error("[Stopped Error]: {message}{location}")]
    Stopped {
        pulses: u32,
        message: &'static str,
        location: ErrorLocation,
    },

    /// A switch tripped during a centring, far from where any should: lost steps.
    #[error("[Switch On The Way Error]: {message}{location}")]
    SwitchOnTheWay {
        switch: Switch,
        pulses: u32,
        message: &'static str,
        location: ErrorLocation,
    },

    /// `seek_timeout_ms` passed with no switch tripping, after this many pulses:
    /// a wiring fault, a missed switch or a motor that is not stepping.
    #[error("[Timed Out Error]: {message}{location}")]
    TimedOut {
        pulses: u32,
        message: &'static str,
        location: ErrorLocation,
    },

    /// The HAL's wake timer refused a wait, for the reason it gives. Every wait
    /// this firmware asks for is far inside the timer's range, so this means a
    /// fault, not a rig condition. A move it ends has stopped first.
    #[error("[Wake Error]: {message}: {error:?}{location}")]
    Wake {
        error: WakeError,
        message: &'static str,
        location: ErrorLocation,
    },

    /// A seek stopped on the other end's switch, so the configured ends or the
    /// wiring are wrong.
    #[error("[Wrong Switch Error]: {message}{location}")]
    WrongSwitch {
        switch: Switch,
        pulses: u32,
        message: &'static str,
        location: ErrorLocation,
    },
}

impl AppError {
    /// The ESP32-S3 bridge did not do what a command asked.
    #[track_caller]
    pub(crate) fn bridge(message: &'static str) -> Self {
        Self::Bridge {
            message,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// The network did not let the board join.
    #[track_caller]
    pub(crate) fn join(message: &'static str) -> Self {
        Self::Join {
            message,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn already_there(switch: Switch) -> Self {
        Self::AlreadyThere {
            switch,
            message: "that end's switch already reads pressed; seek to the other end first",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn bad_speed() -> Self {
        Self::BadSpeed {
            message: BAD_SPEED_MESSAGE,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn not_ready() -> Self {
        Self::NotReady {
            message: "calibration not finished: trip both ends first",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn refused() -> Self {
        Self::Refused {
            message: "both switches read pressed: check the wiring and the USB cable",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn stepper(error: StepError) -> Self {
        Self::Stepper {
            error,
            message: "the step generator refused the move",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn stopped(pulses: u32) -> Self {
        Self::Stopped {
            pulses,
            message: "the move was stopped on request",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn switch_on_the_way(switch: Switch, pulses: u32) -> Self {
        Self::SwitchOnTheWay {
            switch,
            pulses,
            message: "a switch tripped during the centring: steps were lost",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn timed_out(pulses: u32) -> Self {
        Self::TimedOut {
            pulses,
            message: TIMED_OUT_MESSAGE,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn wake(error: WakeError) -> Self {
        Self::Wake {
            error,
            message: "the wake timer refused a wait",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn wrong_switch(switch: Switch, pulses: u32) -> Self {
        Self::WrongSwitch {
            switch,
            pulses,
            message: "the seek stopped on the other end's switch",
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// This failure as a move's answer. The location is left out: it is a long
    /// file path that tells the asker nothing, so only the serial port prints
    /// it. The first three variants are not a move's, and are never answered;
    /// `Stepper` and `Wake` are, as `error`: the reason is printed on the
    /// serial port.
    pub(crate) fn write_answer(&self, out: &mut impl Write) -> fmt::Result {
        let (status, result, message, pin, pulses) = match *self {
            Self::Bridge { message, .. }
            | Self::Format { message, .. }
            | Self::Join { message, .. }
            | Self::Stepper { message, .. }
            | Self::Wake { message, .. } => (STATUS_INTERNAL_ERROR, "error", message, None, 0),
            Self::AlreadyThere {
                switch, message, ..
            } => (STATUS_CONFLICT, "already_there", message, Some(switch), 0),
            Self::BadSpeed { message, .. } => (STATUS_BAD_REQUEST, "bad_speed", message, None, 0),
            Self::NotReady { message, .. } => (STATUS_CONFLICT, "not_ready", message, None, 0),
            Self::Refused { message, .. } => (STATUS_CONFLICT, "refused", message, None, 0),
            Self::Stopped {
                pulses, message, ..
            } => (STATUS_CONFLICT, "stopped", message, None, pulses),
            Self::SwitchOnTheWay {
                switch,
                pulses,
                message,
                ..
            }
            | Self::WrongSwitch {
                switch,
                pulses,
                message,
                ..
            } => (STATUS_CONFLICT, "tripped", message, Some(switch), pulses),
            Self::TimedOut {
                pulses, message, ..
            } => (STATUS_REQUEST_TIMEOUT, "timed_out", message, None, pulses),
        };
        write_answer(out, status, result, message, pin, pulses)
    }
}

impl From<core::fmt::Error> for AppError {
    #[track_caller]
    fn from(_: core::fmt::Error) -> Self {
        Self::Format {
            message: "a message did not fit its buffer",
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
