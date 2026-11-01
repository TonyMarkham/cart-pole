//! The serial line's protocol: text lines, one command or reply per
//! line, so the log reads as it happened. README.md, *The protocol*, gives
//! every line exactly, and the Arduino's firmware must speak the same.

use crate::halt::Halt;

/// A command the manager sends. Rates are pulses/s; pulses and rates are +
/// in Plus, the way the homing's second seek runs.
#[derive(Clone, Debug)]
pub enum Command {
    /// The either-switch homing, then the ramped move to the centre.
    Home,
    /// A slow move of `pulses` at `rate`, stopping early at either switch.
    Move { pulses: i64, rate: u32 },
    /// The ramped move to half the span.
    Centre,
    Enable,
    Disable,
    Stop,
    Status,
}

impl Command {
    /// The command's first word, which its `OK` reply repeats.
    pub const fn word(&self) -> &'static str {
        match self {
            Command::Home => "HOME",
            Command::Move { .. } => "MOVE",
            Command::Centre => "CENTRE",
            Command::Enable => "ENABLE",
            Command::Disable => "DISABLE",
            Command::Stop => "STOP",
            Command::Status => "STATUS",
        }
    }

    /// The line sent, without its newline.
    pub fn line(&self) -> String {
        match self {
            Command::Move { pulses, rate } => format!("MOVE {pulses:+} {rate}"),
            other => other.word().to_owned(),
        }
    }
}

/// A line from the Arduino.
#[derive(Clone, Debug)]
pub enum Line {
    /// `OK <command> <values…>`: the command finished.
    Ok { command: String, values: Vec<String> },
    /// `SWITCH <pin> <count>`: a switch tripped at that count.
    Switch { pin: String, count: i64 },
    /// `HALT <reason> <count>`: the command stopped short.
    Halt(Halt),
    /// `READY`: the firmware has just started, so the board was reset and
    /// everything it knew is gone.
    Ready,
    /// `LOG <text>`: a line for the log. The one just before a `HALT`
    /// says why, in the board's words.
    Log(String),
    /// Any line the protocol does not name: kept in the log only.
    Other,
}

impl Line {
    pub fn parse(text: &str) -> Line {
        let mut words = text.split_whitespace();
        let first = words.next().unwrap_or("");
        let rest: Vec<&str> = words.collect();
        match (first, rest.as_slice()) {
            ("OK", [command, values @ ..]) => Line::Ok {
                command: (*command).to_owned(),
                values: values.iter().map(|value| (*value).to_owned()).collect(),
            },
            ("SWITCH", [pin, count]) => match count.parse() {
                Ok(count) => Line::Switch { pin: (*pin).to_owned(), count },
                Err(_) => Line::Other,
            },
            ("HALT", [reason, count]) => match count.parse() {
                Ok(count) => Line::Halt(Halt { reason: (*reason).to_owned(), count, detail: None }),
                Err(_) => Line::Other,
            },
            ("READY", _) => Line::Ready,
            ("LOG", _) => Line::Log(text.trim_start().trim_start_matches("LOG").trim().to_owned()),
            _ => Line::Other,
        }
    }
}
