use crate::{
    answer::Answer, entry::Entry, error::AppError, heading::Heading, place::Place,
    reading::Reading, switches::Switches, tone::Tone, travel::Travel,
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
    last_trip: Option<Travel>,
    span: Option<u32>,
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

    /// False if a move is already under way, which starts nothing.
    pub(crate) fn begin(&mut self, travel: Travel, now_ms: f64) -> bool {
        if self.is_busy() {
            return false;
        }
        self.heading = match (travel, self.place()) {
            (Travel::Motor, _) | (Travel::Centre, Place::Idler) => Some(Heading::Left),
            (Travel::Idler, _) | (Travel::Centre, Place::Motor) => Some(Heading::Right),
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
                self.centred = false;
            }
            "centred" if answer.succeeded() => {
                self.last_trip = None;
                self.centred = true;
            }
            "already_there" | "bad_speed" | "refused" | "busy" => {}
            "not_ready" => self.last_trip = None,
            "stopped" => {
                self.last_trip = None;
                self.centred = false;
            }
            _ => {
                self.last_trip = None;
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
                "Done: the rail's span is {} pulses, and the cart is at the centre.",
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

/// `59821` as `59,821`.
pub(crate) fn grouped(number: u32) -> String {
    let digits = number.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(status: u16, result: &str, pulses: u32) -> Answer {
        Answer {
            status,
            result: result.to_string(),
            message: "said".to_string(),
            pin: None,
            pulses,
        }
    }

    fn run(session: &mut Session, travel: Travel, status: u16, result: &str, pulses: u32) {
        assert!(session.begin(travel, 1_000.0));
        session.finish(answer(status, result, pulses), 2_000.0, "now".to_string());
    }

    fn levels(d2: u8, d3: u8) -> Reading {
        Reading::Levels(Switches { d2, d3 })
    }

    #[test]
    fn the_span_needs_a_trip_at_each_end() {
        let mut session = Session::new();
        run(&mut session, Travel::Idler, 200, "tripped", 29_920);
        assert_eq!(session.span, None);
        run(&mut session, Travel::Motor, 200, "tripped", 59_828);
        assert_eq!(session.span, Some(59_828));
    }

    #[test]
    fn the_same_end_twice_measures_nothing() {
        let mut session = Session::new();
        run(&mut session, Travel::Idler, 200, "tripped", 100);
        run(&mut session, Travel::Idler, 200, "tripped", 200);
        assert_eq!(session.span, None);
    }

    #[test]
    fn a_timeout_or_lost_answer_forgets_the_span() {
        for result in ["timed_out", "no_answer"] {
            let mut session = Session::new();
            run(&mut session, Travel::Idler, 200, "tripped", 100);
            run(&mut session, Travel::Motor, 200, "tripped", 59_828);
            run(&mut session, Travel::Idler, 408, result, 0);
            assert_eq!(session.span, None, "{result}");
            assert_eq!(session.last_trip, None, "{result}");
        }
    }

    #[test]
    fn an_answer_that_moved_nothing_changes_nothing() {
        for result in ["already_there", "bad_speed", "refused", "busy"] {
            let mut session = Session::new();
            run(&mut session, Travel::Idler, 200, "tripped", 100);
            run(&mut session, Travel::Motor, 200, "tripped", 59_828);
            run(&mut session, Travel::Motor, 409, result, 0);
            assert_eq!(session.span, Some(59_828), "{result}");
            assert_eq!(session.last_trip, Some(Travel::Motor), "{result}");
        }
    }

    #[test]
    fn a_stop_keeps_the_span_but_not_the_switch() {
        let mut session = Session::new();
        run(&mut session, Travel::Idler, 200, "tripped", 100);
        run(&mut session, Travel::Motor, 200, "tripped", 59_828);
        run(&mut session, Travel::Idler, 409, "stopped", 1_000);
        assert_eq!(session.span, Some(59_828));
        assert_eq!(session.last_trip, None);
        assert!(session.next_step().contains("Find either end"));
    }

    #[test]
    fn a_stop_after_a_centring_leaves_the_cart_unknown() {
        let mut session = Session::new();
        run(&mut session, Travel::Centre, 200, "centred", 29_910);
        run(&mut session, Travel::Idler, 409, "stopped", 1_000);
        session.read(levels(0, 0), "now".to_string());
        assert_eq!(session.place(), Place::Unknown);
    }

    #[test]
    fn the_stop_line_lasts_only_while_the_move_does() {
        let mut session = Session::new();
        assert!(session.begin(Travel::Motor, 0.0));
        session.stop_asked(None);
        assert_eq!(session.status(0.0).0, Tone::Warn);
        assert!(session.status(0.0).1.contains("Stop sent"));
        session.finish(answer(409, "stopped", 10), 1.0, "now".to_string());
        assert!(!session.status(0.0).1.contains("Stop sent"));
    }

    #[test]
    fn a_stop_that_arrives_after_the_move_says_nothing() {
        let mut session = Session::new();
        session.stop_asked(None);
        assert!(!session.status(0.0).1.contains("Stop sent"));
    }

    #[test]
    fn a_tripped_answer_on_the_wrong_switch_is_no_trip() {
        let mut session = Session::new();
        run(&mut session, Travel::Idler, 200, "tripped", 100);
        run(&mut session, Travel::Motor, 409, "tripped", 50);
        assert_eq!(session.span, None);
        assert_eq!(session.last_trip, None);
    }

    #[test]
    fn the_centre_lasts_until_a_switch_reads_pressed() {
        let mut session = Session::new();
        run(&mut session, Travel::Centre, 200, "centred", 29_910);
        session.read(levels(0, 0), "now".to_string());
        assert_eq!(session.place(), Place::Centre);
        session.read(levels(1, 0), "now".to_string());
        assert_eq!(session.place(), Place::Motor);
        session.read(levels(0, 0), "now".to_string());
        assert_eq!(session.place(), Place::Unknown);
    }

    #[test]
    fn both_switches_pressed_is_no_place() {
        let mut session = Session::new();
        session.read(levels(1, 1), "now".to_string());
        assert_eq!(session.place(), Place::BothPressed);
    }

    #[test]
    fn a_reading_that_lands_during_a_move_is_ignored() {
        let mut session = Session::new();
        assert!(session.begin(Travel::Motor, 0.0));
        session.read(levels(1, 0), "now".to_string());
        assert_eq!(session.switches(), None);
        assert_eq!(session.place(), Place::Moving);
    }

    #[test]
    fn a_move_drops_the_last_reading() {
        let mut session = Session::new();
        session.read(levels(0, 0), "now".to_string());
        assert!(session.begin(Travel::Motor, 0.0));
        assert_eq!(session.switches(), None);
    }

    #[test]
    fn a_second_move_does_not_start() {
        let mut session = Session::new();
        assert!(session.begin(Travel::Motor, 0.0));
        assert!(!session.begin(Travel::Idler, 0.0));
        session.finish(answer(200, "tripped", 1), 1.0, "now".to_string());
        session.read(Reading::MovingElsewhere, "now".to_string());
        assert!(!session.begin(Travel::Idler, 0.0));
    }

    #[test]
    fn a_centring_heads_away_from_the_switch_it_rests_on() {
        let mut session = Session::new();
        session.read(levels(1, 0), "now".to_string());
        assert!(session.begin(Travel::Centre, 0.0));
        assert_eq!(session.heading(), Some(Heading::Right));
    }

    #[test]
    fn numbers_are_grouped_in_threes() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(59_821), "59,821");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }
}
