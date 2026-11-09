pub(crate) mod address;
pub(crate) mod at;
pub(crate) mod datagram;
pub(crate) mod deadline;
pub mod error;
mod frame_reader;
mod frame_step;
pub(crate) mod join;
pub(crate) mod reply;
pub(crate) mod udp_socket;

// ---------------------------------------------------------------------------------------------- //

use crate::{sleep, wifi::deadline::Deadline};

use unor4_hal::{
    uart::UartIrq,
    wake::{WakeError, WakeTimer},
};

/// The link to the WiFi chip: SCI1, as in the controller.
pub(crate) static AT: UartIrq<ra4m1::SCI1> = UartIrq::new();

/// Sleeps until the WiFi chip sends a byte, and returns it, or None once
/// `deadline` has passed. Either one ends the sleep: a byte through the link's
/// receive interrupt, the deadline through the wake timer's. A byte already
/// waiting is returned even if the deadline has passed. Only the join waits
/// this way; the run loop never waits on the chip.
pub(crate) fn next_byte(
    wake: &mut WakeTimer,
    deadline: &mut Deadline,
) -> Result<Option<u8>, WakeError> {
    sleep::until(|| {
        if let Some(byte) = AT.read_byte() {
            return Some(Ok(Some(byte)));
        }
        match deadline.passed(wake) {
            Ok(true) => Some(Ok(None)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        }
    })
}
