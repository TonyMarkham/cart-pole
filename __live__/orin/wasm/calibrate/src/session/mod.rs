pub(crate) mod entry;
pub(crate) mod heading;
pub(crate) mod place;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    board::{answer::Answer, reading::Reading, switches::Switches, travel::Travel},
    error::app_error::AppError,
    number::grouped,
    session::{entry::Entry, heading::Heading, place::Place},
    tone::Tone,
};

/// What this page knows, from what it has asked since it was opened. The board
/// has the final word on all of it: a refreshed page knows nothing, though the
/// board remembers, and a move it refuses is simply answered with why.
pub(crate) struct Session {
    pub(crate) address: String,
    pub(crate) entries: Vec<Entry>,
    switches: Option<Switches>,
    /// Why the last reading failed; None before the first, or after a good one.
    reading_failed: Option<String>,
    reading_clock: Option<String>,
    moving: Option<Travel>,
    heading: Option<Heading>,
    moving_elsewhere: bool,
    started_ms: f64,
    /// What came of pressing STOP, until the move has ended.
    stop_line: Option<(Tone, String)>,
    /// The end the cart last tripped, as the board's `resting_on` does.
    pub(crate) last_trip: Option<Travel>,
    /// Where the last move ended as asked, so where the cart can be measured.
    last_stop: Option<Travel>,
    pub(crate) span: Option<u32>,
    centred: bool,
}

impl Session {
    pub(crate) fn new() -> Self {
        Self {
            address: String::new(),
            entries: Vec::new(),
            switches: None,
            reading_failed: None,
            reading_clock: None,
            moving: None,
            heading: None,
            moving_elsewhere: false,
            started_ms: 0.0,
            stop_line: None,
            last_trip: None,
            last_stop: None,
            span: None,
            centred: false,
        }
    }

    pub(crate) fn is_moving(&self) -> bool {
        self.moving.is_some()
    }

    /// Whether a move is under way: the move buttons are off, because a second
    /// move would be thrown away, and STOP is the only one on.
    pub(crate) fn is_busy(&self) -> bool {
        self.moving.is_some() || self.moving_elsewhere
    }

    pub(crate) fn switches(&self) -> Option<Switches> {
        self.switches
    }

    pub(crate) fn heading(&self) -> Option<Heading> {
        self.heading
    }

    /// The stop the cart can be measured at now, if any.
    pub(crate) fn stop_here(&self) -> Option<Travel> {
        self.last_stop
    }

    /// What the webserver remembers, for a page that has just been opened.
    pub(crate) fn restore(&mut self, last_stop: Option<Travel>, span: Option<u32>) {
        self.last_stop = last_stop;
        self.last_trip = last_stop.filter(|&stop| stop != Travel::Centre);
        self.centred = last_stop == Some(Travel::Centre);
        self.span = span;
    }

    /// False if a move is already under way, which starts nothing.
    pub(crate) fn begin(&mut self, travel: Travel, now_ms: f64) -> bool {
        if self.is_busy() {
            return false;
        }
        self.heading = match (travel, self.place()) {
            (Travel::Idler, _) | (Travel::Centre, Place::Motor) => Some(Heading::Left),
            (Travel::Motor, _) | (Travel::Centre, Place::Idler) => Some(Heading::Right),
            (Travel::Centre, _) => None,
        };
        // The last reading says nothing once the cart has set off.
        self.switches = None;
        self.moving = Some(travel);
        self.started_ms = now_ms;
        true
    }

    /// What came of pressing STOP: `None` if the webserver sent it, or why it
    /// could not. Too late to matter if the move has ended meanwhile.
    pub(crate) fn stop_asked(&mut self, failed: Option<AppError>) {
        if !self.is_busy() {
            return;
        }
        self.stop_line = Some(match failed {
            None => (
                Tone::Warn,
                "Stop sent. If the cart keeps moving, press STOP again or use the e-stop."
                    .to_string(),
            ),
            Some(reason) => (
                Tone::Warn,
                format!("The stop could not be sent: use the e-stop. Reason: {reason}"),
            ),
        });
    }

    /// Takes in a move's answer, keeping what the board's `Rail` keeps: the span
    /// counts only from a trip it saw to a trip at the other end. An answer that
    /// says nothing moved changes nothing. A stop loses no steps, so the span
    /// stands, but the cart may have left its switch. Any other failure, a lost
    /// answer included, leaves the page unsure, so it forgets, as the board does
    /// for a wrong switch or a timeout.
    pub(crate) fn finish(&mut self, answer: Answer, now_ms: f64, clock: String) {
        let Some(travel) = self.moving.take() else {
            return;
        };
        self.heading = None;
        self.stop_line = None;
        match answer.result.as_str() {
            "tripped" if answer.succeeded() => {
                if self.last_trip.is_some_and(|last| last != travel) {
                    self.span = Some(answer.pulses);
                }
                self.last_trip = Some(travel);
                self.last_stop = Some(travel);
                self.centred = false;
            }
            "centred" if answer.succeeded() => {
                self.last_trip = None;
                self.last_stop = Some(travel);
                self.centred = true;
            }
            "already_there" | "bad_speed" | "refused" | "busy" => {}
            "not_ready" => self.last_trip = None,
            "stopped" => {
                self.last_trip = None;
                self.last_stop = None;
                self.centred = false;
            }
            _ => {
                self.last_trip = None;
                self.last_stop = None;
                self.span = None;
                self.centred = false;
            }
        }
        self.entries.push(Entry {
            clock,
            travel,
            answer,
            took_s: (now_ms - self.started_ms) / 1_000.0,
        });
    }

    /// Takes in a reading, unless a move began while it was on its way: the board
    /// throws away what reaches it during a move, so that says nothing.
    pub(crate) fn read(&mut self, reading: Reading, clock: String) {
        if self.moving.is_some() {
            return;
        }
        self.moving_elsewhere = false;
        match reading {
            Reading::Levels(switches) => {
                self.centred &= !switches.motor_pressed() && !switches.idler_pressed();
                self.switches = Some(switches);
                self.stop_line = None;
                self.reading_failed = None;
                self.reading_clock = Some(clock);
            }
            Reading::MovingElsewhere => {
                self.switches = None;
                self.moving_elsewhere = true;
            }
            Reading::Failed(reason) => {
                self.switches = None;
                self.reading_failed = Some(reason);
            }
        }
    }

    pub(crate) fn place(&self) -> Place {
        if self.is_busy() {
            return Place::Moving;
        }
        match self
            .switches
            .map(|switches| (switches.motor_pressed(), switches.idler_pressed()))
        {
            Some((true, true)) => Place::BothPressed,
            Some((true, false)) => Place::Motor,
            Some((false, true)) => Place::Idler,
            Some((false, false)) | None if self.centred => Place::Centre,
            _ => Place::Unknown,
        }
    }

    /// The line about the connection, or the move under way.
    pub(crate) fn status(&self, now_ms: f64) -> (Tone, String) {
        if let Some(line) = &self.stop_line {
            return line.clone();
        }
        if let Some(travel) = self.moving {
            let so_far = ((now_ms - self.started_ms) / 1_000.0).max(0.0);
            return (
                Tone::Info,
                format!(
                    "Moving {}: {so_far:.0} s so far. The board answers when the cart has stopped.",
                    travel.heading()
                ),
            );
        }
        if self.moving_elsewhere {
            return (
                Tone::Info,
                "A move is under way, started by another page or before this one was refreshed. \
                 STOP works on it. The other buttons come back when it has ended."
                    .to_string(),
            );
        }
        if let Some(reason) = &self.reading_failed {
            return (
                Tone::Warn,
                format!(
                    "No answer from the board at {}. It may be off, running other firmware, or its \
                     answer may have been lost on the WiFi, in which case the next reading will \
                     show. Reason: {reason}",
                    self.address
                ),
            );
        }
        match &self.reading_clock {
            Some(clock) => (
                Tone::Ok,
                format!("The board at {} answered at {clock}.", self.address),
            ),
            None => (
                Tone::None,
                "Asking the board for its first reading.".to_string(),
            ),
        }
    }

    /// What to do next, as far as this page can tell.
    pub(crate) fn next_step(&self) -> String {
        if self.is_busy() {
            return "Wait for the cart to stop. Press STOP if anything looks wrong, and use the \
                    e-stop if that does not stop it."
                .to_string();
        }
        match (self.span, self.last_trip) {
            (Some(span), _) if self.centred => format!(
                "The rail's span is {} pulses, and the cart is at the centre. Measure its pose, and                  the three at each end, to fit the camera.",
                grouped(span)
            ),
            (Some(span), Some(_)) => format!(
                "Both ends are found: the rail's span is {} pulses. You can move to the centre.",
                grouped(span)
            ),
            (Some(span), None) => format!(
                "The rail's span is {} pulses, but the centring starts from a switch. Find \
                 either end first.",
                grouped(span)
            ),
            (None, Some(Travel::Motor)) => {
                "One end is found. Now find the idler end: that move measures the rail's span."
                    .to_string()
            }
            (None, Some(Travel::Idler)) => {
                "One end is found. Now find the motor end: that move measures the rail's span."
                    .to_string()
            }
            _ => "Start by finding either end. If the board answers already_there, find the other \
                  end first."
                .to_string(),
        }
    }

    /// The line about the last move, if there has been one.
    pub(crate) fn last_result(&self) -> Option<(Tone, String)> {
        let entry = self.entries.last()?;
        let answer = &entry.answer;
        let tone = if answer.succeeded() {
            Tone::Ok
        } else {
            Tone::Warn
        };
        let how_far = if answer.pulses > 0 {
            format!(
                " after {} pulses and {:.1} s",
                grouped(answer.pulses),
                entry.took_s
            )
        } else {
            String::new()
        };
        Some((
            tone,
            format!(
                "{}: {}{how_far}. It said: {}.",
                entry.travel.name(),
                answer.result,
                answer.message
            ),
        ))
    }
}
