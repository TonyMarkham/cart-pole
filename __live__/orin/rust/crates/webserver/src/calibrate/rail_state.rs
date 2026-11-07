use crate::{calibrate::stop::Stop, constants::STATUS_OK};

use serde_json::Value;

/// What the moves' answers have said about the rail, because the board has no
/// message that reports its span. It follows the board's `Rail`: a span counts only
/// from a trip at one end to a trip at the other. It is lost when the server
/// restarts, and it cannot see a board that has reset.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RailState {
    /// Where the last move ended as asked, so where the cart can be measured.
    pub(crate) last_stop: Option<Stop>,
    /// Pulses between the two switches.
    pub(crate) span: Option<u32>,
    /// The end the cart last tripped and has not left.
    last_trip: Option<Stop>,
}

impl RailState {
    /// Takes in a move's answer. An answer that says nothing moved changes nothing.
    /// A stop loses no steps, so the span stands, but the cart has left its stop.
    /// Any other failure leaves the rail unknown, as it does on the board.
    pub(crate) fn note(&mut self, stop: Stop, answer: &[u8]) {
        let answer = serde_json::from_slice::<Value>(answer).unwrap_or_default();
        let worked = answer["status"].as_u64() == Some(STATUS_OK);
        let pulses = answer["pulses"]
            .as_u64()
            .and_then(|pulses| u32::try_from(pulses).ok());
        match answer["result"].as_str().unwrap_or_default() {
            "tripped" if worked => {
                if self.last_trip.is_some_and(|last| last != stop) {
                    self.span = pulses;
                }
                self.last_trip = Some(stop);
                self.last_stop = Some(stop);
            }
            "centred" if worked => {
                self.last_trip = None;
                self.last_stop = Some(Stop::Centre);
            }
            "already_there" | "bad_speed" | "refused" => {}
            "not_ready" => self.last_trip = None,
            "stopped" => {
                self.last_trip = None;
                self.last_stop = None;
            }
            _ => self.forget(),
        }
    }

    /// A move with no answer: the cart may have moved anywhere.
    pub(crate) fn forget(&mut self) {
        *self = Self::default();
    }
}
