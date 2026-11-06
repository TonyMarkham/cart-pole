pub mod answer;
mod answer_buffer;
pub(crate) mod asker;
pub(crate) mod at;
pub(crate) mod datagram;
pub(crate) mod deadline;
pub mod error;
mod frame_reader;
mod frame_step;
pub(crate) mod join;
pub(crate) mod message;
pub(crate) mod reply;
pub(crate) mod udp_socket;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    sleep,
    state::State,
    wifi::{
        answer::Answer, answer_buffer::AnswerBuffer, asker::Asker, deadline::Deadline,
        message::Message as WifiMessage, udp_socket::UdpSocket,
    },
};

use unor4_hal::{
    uart::UartIrq,
    wake::{WakeError, WakeTimer},
};

/// (AT, as it is now.)
pub(crate) static AT: UartIrq<ra4m1::SCI1> = UartIrq::new();

/// Sleeps until the WiFi chip sends a byte, and returns it, or None once
/// `deadline` has passed. Either one ends the sleep: a byte through the link's
/// receive interrupt, the deadline through the wake timer's. A byte already
/// waiting is returned even if the deadline has passed.
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

/// Answers `message` as the board stands in `state`: `state` gets the state,
/// a command gets the reply for a command the board doesn't apply, and any
/// other datagram gets nothing. A seek isn't answered here: it moves the cart
/// (`main`), and `Stop` outside a move does nothing.
pub(crate) fn serve(
    udp: &mut UdpSocket,
    wake: &mut WakeTimer,
    state: State,
    message: Option<WifiMessage>,
) -> Result<(), WakeError> {
    let answer = match message {
        Some(WifiMessage::State) => Answer::state(state),
        Some(WifiMessage::Command) => match Answer::command_refused(state) {
            Some(answer) => answer,
            None => return Ok(()),
        },
        Some(WifiMessage::Seek(_) | WifiMessage::Stop) | None => return Ok(()),
    };
    match udp.asker(wake)? {
        Some(asker) => send_to(udp, wake, &asker, &answer),
        None => Ok(()),
    }
}

/// Sends `answer` to `asker`. An answer that can't be sent is dropped, and the
/// Orin's wait for it times out.
pub(crate) fn send_to(
    udp: &mut UdpSocket,
    wake: &mut WakeTimer,
    asker: &Asker,
    answer: &Answer,
) -> Result<(), WakeError> {
    let mut text = AnswerBuffer::new();
    // Never fails: ANSWER_MAX has room for the longest answer.
    if answer.write(&mut text).is_err() {
        return Ok(());
    }

    let _sent = udp.send_to(wake, asker, text.as_bytes())?;
    Ok(())
}
