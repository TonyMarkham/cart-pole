//! What the Pi knows about the rig, and the actions the page's buttons
//! start. One action runs at a time; STOP is sent at once, whatever is
//! running.

use std::{
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::sync::broadcast::error::RecvError;

use crate::{
    experiment::{self, ExperimentReport},
    halt::Halt,
    link::{Event, Link},
    log::{LogBook, Source},
    protocol::{Command, Line},
    settings::Settings,
    trials::TrialBook,
};

/// What a button asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Calibrate,
    Centre,
    JogMinus,
    JogPlus,
    Enable,
    Disable,
    Status,
    Experiment,
    Flash,
    HardReset,
}

/// The baud that makes the bridge reset the Arduino's processor when the
/// host opens the port at it: one pulse on its reset line with BOOT low,
/// a normal start, as one tap of the reset button
/// (`UNOR4USBBridge.ino`, `usbEventCallback`; 1,200 is the double tap
/// into the bootloader).
const RESET_BAUD: u32 = 2_400;

impl Action {
    /// The buttons, in the page's order. Flash has none: it is a POST of
    /// the .bin.
    pub const BUTTONS: [Action; 8] = [
        Action::Calibrate,
        Action::Centre,
        Action::JogMinus,
        Action::JogPlus,
        Action::Enable,
        Action::Disable,
        Action::Status,
        Action::Experiment,
    ];

    pub const fn path(self) -> &'static str {
        match self {
            Action::Calibrate => "/calibrate",
            Action::Centre => "/centre",
            Action::JogMinus => "/jog-minus",
            Action::JogPlus => "/jog-plus",
            Action::Enable => "/enable",
            Action::Disable => "/disable",
            Action::Status => "/status",
            Action::Experiment => "/experiment",
            Action::Flash => "/flash",
            Action::HardReset => "/hard-reset-arduino",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Action::Calibrate => "Calibrate",
            Action::Centre => "Go to centre",
            Action::JogMinus => "Jog minus",
            Action::JogPlus => "Jog plus",
            Action::Enable => "Enable motor",
            Action::Disable => "Disable motor",
            Action::Status => "Ask the board",
            Action::Experiment => "Jump-limit experiment",
            Action::Flash => "Flash",
            Action::HardReset => "HARD RESET ARDUINO",
        }
    }
}

/// The rig's facts, from the Arduino's replies.
#[derive(Clone, Debug, Default)]
pub struct Rig {
    /// The pin that tripped at the first end, the reference switch.
    pub first: Option<String>,
    pub second: Option<String>,
    /// Pulses from the first end to the second.
    pub span: Option<u32>,
    /// Whether the count says where the cart is: true from a finished
    /// homing until the driver goes off or the board resets.
    pub homed: bool,
    /// The count the last reply gave.
    pub position: Option<i64>,
}

#[derive(Debug, Default)]
pub struct State {
    pub rig: Rig,
    pub busy: Option<Action>,
    /// The last action and how it ended, in words.
    pub last: Option<(Action, String)>,
    /// A halt to show at the top of the page until the next homing: a
    /// fault that switched the driver off, or a move the board refused.
    pub warning: Option<Warning>,
    pub experiment: ExperimentReport,
}

/// A halt shown at the top of the page.
#[derive(Clone, Debug)]
pub struct Warning {
    /// Whether the board switched the driver off: the cart is free and
    /// must be homed again.
    pub fault: bool,
    pub text: String,
}

/// Why a command or an action did not finish.
#[derive(Clone, Debug)]
pub enum Failure {
    Halted(Halt),
    /// The board reset (`READY`) while the command ran.
    Reset,
    /// The port closed while the command ran.
    Lost,
    /// The port is not open.
    Closed,
    /// No reply within `reply_timeout_ms`.
    Timeout(&'static str),
    /// A reply the protocol does not allow.
    Protocol(String),
    NotHomed,
    /// STOP was pressed while the Pi waited between commands.
    Stopped,
}

impl Failure {
    /// Whether the experiment can re-home and carry on: the board went
    /// away or restarted, which loses its state but not the run's.
    pub fn resumable(&self) -> bool {
        matches!(self, Failure::Reset | Failure::Lost | Failure::Closed)
    }

    pub fn describe(&self) -> String {
        match self {
            Failure::Halted(halt) => halt.describe(),
            Failure::Reset => "the board reset while it ran: home it again".to_owned(),
            Failure::Lost => "the serial line closed while it ran".to_owned(),
            Failure::Closed => "the serial line is not open: is the board plugged in?".to_owned(),
            Failure::Timeout(word) => format!("no reply to {word} in time: the board has stopped answering"),
            Failure::Protocol(line) => format!("a reply the Pi does not understand: {line}"),
            Failure::NotHomed => "not homed yet: press Calibrate first".to_owned(),
            Failure::Stopped => "STOP pressed".to_owned(),
        }
    }
}

/// A finished command's reply: the values after `OK <command>`, and the
/// switch that tripped on the way, if one did.
#[derive(Debug, Default)]
pub struct Reply {
    pub values: Vec<String>,
    pub switch: Option<(String, i64)>,
}

impl Reply {
    /// The `index`-th value as a number.
    pub fn number(&self, index: usize, word: &str) -> Result<i64, Failure> {
        self.values
            .get(index)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| Failure::Protocol(format!("OK {word} {}", self.values.join(" "))))
    }

    pub fn text(&self, index: usize, word: &str) -> Result<String, Failure> {
        self.values
            .get(index)
            .cloned()
            .ok_or_else(|| Failure::Protocol(format!("OK {word} {}", self.values.join(" "))))
    }
}

pub struct Station {
    pub settings: Settings,
    pub link: Link,
    pub log: Arc<Mutex<LogBook>>,
    /// The trial records the Arduino sends (`POST /trial`).
    pub trials: Mutex<TrialBook>,
    state: Mutex<State>,
    stop: AtomicBool,
}

impl Station {
    pub fn new(settings: Settings, link: Link, log: Arc<Mutex<LogBook>>) -> Self {
        let trials = Mutex::new(TrialBook::new(&settings.station.log_dir));
        Self { settings, link, log, trials, state: Mutex::new(State::default()), stop: AtomicBool::new(false) }
    }

    pub fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A line of the Pi's own in the log.
    pub fn note(&self, text: &str) {
        self.log.lock().unwrap_or_else(PoisonError::into_inner).write(Source::Note, text);
    }

    /// Claims the station for `action`; false if another is running.
    pub fn claim(&self, action: Action) -> bool {
        let mut state = self.state();
        if state.busy.is_some() {
            return false;
        }
        state.busy = Some(action);
        self.stop.store(false, Ordering::SeqCst);
        true
    }

    /// Ends the running action with its outcome, noted in the log.
    pub fn finish(&self, action: Action, outcome: String) {
        self.note(&format!("{}: {outcome}", action.label()));
        self.release(action, outcome);
    }

    /// Ends the running action with its outcome, without a note.
    pub fn release(&self, action: Action, outcome: String) {
        let mut state = self.state();
        state.busy = None;
        state.last = Some((action, outcome));
    }

    /// Sends STOP now, whatever is running, and stops the Pi's own waits.
    pub async fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.note("STOP pressed");
        if !self.link.send(Command::Stop.line()).await {
            self.note("STOP not sent: the serial link has ended");
        }
    }

    pub fn stop_pressed(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Sends one command and waits for its reply. A `HALT` that freed the
    /// cart, or a reset, forgets the homing.
    pub async fn command(&self, command: &Command) -> Result<Reply, Failure> {
        let mut events = self.link.subscribe();
        if !self.link.connected() {
            return Err(Failure::Closed);
        }
        if !self.link.send(command.line()).await {
            return Err(Failure::Closed);
        }
        let word = command.word();
        let deadline = tokio::time::Instant::now() + Duration::from_millis(self.settings.serial.reply_timeout_ms);
        let mut reply = Reply::default();
        // The board's last log line, which says why when a HALT follows.
        let mut said: Option<String> = None;
        let result = loop {
            let event = match tokio::time::timeout_at(deadline, events.recv()).await {
                Ok(event) => event,
                Err(_) => break Err(Failure::Timeout(word)),
            };
            match event {
                Ok(Event::Line(Line::Ok { command, values })) if command == word => {
                    reply.values = values;
                    break Ok(reply);
                }
                Ok(Event::Line(Line::Switch { pin, count })) => reply.switch = Some((pin, count)),
                Ok(Event::Line(Line::Halt(mut halt))) => {
                    halt.detail = said.take();
                    break Err(Failure::Halted(halt));
                }
                // The board starts a halt's line with what it did
                // ("stopped: " or "FAULT, driver off: "); the page says
                // that itself, so only the reason is kept.
                Ok(Event::Line(Line::Log(text))) => {
                    let reason = ["stopped: ", "FAULT, driver off: "]
                        .iter()
                        .find_map(|prefix| text.strip_prefix(prefix))
                        .unwrap_or(&text);
                    said = Some(reason.to_owned());
                }
                Ok(Event::Line(Line::Ready)) => break Err(Failure::Reset),
                Ok(Event::Lost) | Err(RecvError::Closed) => break Err(Failure::Lost),
                Ok(Event::Line(_)) => {}
                Err(RecvError::Lagged(missed)) => self.note(&format!("missed {missed} lines while waiting for {word}")),
            }
        };
        match &result {
            Err(Failure::Halted(halt)) if halt.frees_cart() => {
                let mut state = self.state();
                state.rig.homed = false;
                state.warning = Some(Warning { fault: true, text: halt.describe() });
            }
            Err(Failure::Halted(halt)) => {
                let mut state = self.state();
                state.rig.position = Some(halt.count);
                if halt.reason == "into-switch" || halt.reason == "pressed-at-start" {
                    state.warning = Some(Warning { fault: false, text: halt.describe() });
                }
            }
            Err(Failure::Reset | Failure::Lost) => self.state().rig = Default::default(),
            _ => {}
        }
        result
    }

    /// Waits `ms`, unless STOP is pressed.
    pub async fn pause(&self, ms: u64) -> Result<(), Failure> {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        if self.stop_pressed() { Err(Failure::Stopped) } else { Ok(()) }
    }

    /// `HOME`: replies `OK HOME <first pin> <second pin> <span> <position>`.
    pub async fn home(&self) -> Result<String, Failure> {
        let reply = self.command(&Command::Home).await?;
        let first = reply.text(0, "HOME")?;
        let second = reply.text(1, "HOME")?;
        let span = u32::try_from(reply.number(2, "HOME")?).map_err(|_| Failure::Protocol("HOME span".to_owned()))?;
        let position = reply.number(3, "HOME")?;
        let text = format!("first end {first}, second end {second}, span {span} pulses; centred at pulse {position}");
        let mut state = self.state();
        state.rig = Rig { first: Some(first), second: Some(second), span: Some(span), homed: true, position: Some(position) };
        state.warning = None;
        drop(state);
        Ok(text)
    }

    /// A slow move; replies `OK MOVE <position>`, after `SWITCH <pin>
    /// <count>` if a switch stopped it early. Returns that switch.
    pub async fn slow_move(&self, pulses: i64) -> Result<Option<(String, i64)>, Failure> {
        let reply = self.command(&Command::Move { pulses, rate: self.settings.rig.seek_speed }).await?;
        self.state().rig.position = Some(reply.number(0, "MOVE")?);
        Ok(reply.switch)
    }

    /// `SEEK REF`: replies `OK SEEK <count>`, the count the reference
    /// switch tripped at.
    pub async fn seek_ref(&self) -> Result<i64, Failure> {
        let count = self.command(&Command::SeekRef).await?.number(0, "SEEK")?;
        self.state().rig.position = Some(count);
        Ok(count)
    }

    /// Forgets the rig's facts whenever the board says it has just
    /// started (`READY`) or its port closes, whatever is running: the
    /// board's homing is gone either way.
    pub fn watch_resets(self: &Arc<Self>) {
        let station = Arc::clone(self);
        let mut events = self.link.subscribe();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(Event::Line(Line::Ready)) => {
                        station.note("the board has just started: its homing is gone");
                        station.state().rig = Rig::default();
                    }
                    Ok(Event::Lost) => station.state().rig = Rig::default(),
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => return,
                }
            }
        });
    }

    /// Forgets everything the page shows from before (the warning, the
    /// last result) and asks the board for STATUS, so the page holds only
    /// what the board says now. False if an action is running: the board
    /// takes one command at a time.
    pub fn reset(self: &Arc<Self>) -> bool {
        {
            let mut state = self.state();
            if state.busy.is_some() {
                return false;
            }
            state.warning = None;
            state.last = None;
        }
        self.note("Reset: cleared the page; asking the board");
        self.start(Action::Status)
    }

    /// HARD RESET ARDUINO: restarts the Arduino, as one tap of its reset
    /// button. The link closes the port, it is opened once at
    /// `RESET_BAUD`, which makes the bridge pulse the reset line, and the
    /// link reopens it at the line's own baud. It runs whatever else is
    /// running, since it is the way out of a board that has stopped
    /// answering; a running command ends as the port closes. Refused only
    /// during a flash, which owns the port. The board forgets its homing,
    /// and nothing moves: the firmware waits for a command after `READY`.
    ///
    /// The `READY` itself rarely arrives: after every baud change the
    /// bridge empties what the Arduino has sent it
    /// (`UNOR4USBBridge.ino`, `usbEventCallback`), and the reset's boot
    /// falls inside that. So 1 s later the Pi asks STATUS, and the page
    /// shows the board answering.
    pub async fn hard_reset(self: &Arc<Self>) -> String {
        if self.state().busy == Some(Action::Flash) {
            return "refused: a flash is running".to_owned();
        }
        let port = self.settings.serial.port.clone();
        self.note(&format!("HARD RESET ARDUINO: opening {port} at {RESET_BAUD} baud"));
        self.link.pause().await;
        let opened = tokio::task::spawn_blocking({
            let port = port.clone();
            move || tokio_serial::new(&port, RESET_BAUD).open().map(drop)
        })
        .await;
        let outcome = match opened {
            Ok(Ok(())) => "the Arduino was reset; asking it STATUS in 1 s".to_owned(),
            Ok(Err(error)) => format!("failed: cannot open {port} at {RESET_BAUD} baud: {error}"),
            Err(error) => format!("failed: {error}"),
        };
        self.link.resume().await;
        self.note(&format!("HARD RESET ARDUINO: {outcome}"));
        {
            let mut state = self.state();
            state.rig = Rig::default();
            state.warning = None;
            state.last = Some((Action::HardReset, outcome.clone()));
        }
        if outcome.starts_with("the Arduino was reset") {
            let station = Arc::clone(self);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if !station.start(Action::Status) {
                    station.note("HARD RESET ARDUINO: STATUS not asked, an action is running");
                }
            });
        }
        outcome
    }

    /// Starts `action` in the background; false if another is running.
    pub fn start(self: &Arc<Self>, action: Action) -> bool {
        if !self.claim(action) {
            return false;
        }
        self.note(action.label());
        let station = Arc::clone(self);
        tokio::spawn(async move {
            let outcome = match station.run(action).await {
                Ok(text) => text,
                Err(failure) => format!("stopped: {}", failure.describe()),
            };
            station.finish(action, outcome);
        });
        true
    }

    async fn run(&self, action: Action) -> Result<String, Failure> {
        match action {
            Action::Calibrate => self.home().await,
            Action::Centre => {
                let reply = self.command(&Command::Centre).await?;
                let position = reply.number(0, "CENTRE")?;
                self.state().rig.position = Some(position);
                Ok(format!("centred at pulse {position}"))
            }
            Action::JogMinus | Action::JogPlus => {
                let pulses = i64::from(self.settings.rig.jog_pulses);
                let pulses = if action == Action::JogMinus { -pulses } else { pulses };
                let position = self.state().rig.position;
                match self.slow_move(pulses).await? {
                    Some((pin, count)) => Ok(format!("jog stopped at pulse {count}: {pin} tripped")),
                    None => Ok(format!(
                        "jogged {pulses:+} pulses from {} to {}",
                        position.map_or("?".to_owned(), |p| p.to_string()),
                        self.state().rig.position.map_or("?".to_owned(), |p| p.to_string())
                    )),
                }
            }
            Action::Enable => self.command(&Command::Enable).await.map(|_| "driver enabled".to_owned()),
            Action::Disable => {
                self.command(&Command::Disable).await?;
                self.state().rig.homed = false;
                Ok("driver off: the cart is free and must be homed again".to_owned())
            }
            // `OK STATUS homed=… position=… first=… span=… d2=… …`: the
            // rig's facts are taken from it, so the Pi's copy is the
            // board's now, and the whole answer is shown as it came.
            Action::Status => {
                let reply = self.command(&Command::Status).await?;
                let value = |name: &str| {
                    reply.values.iter().find_map(|word| word.strip_prefix(name)?.strip_prefix('='))
                };
                {
                    let mut state = self.state();
                    let rig = &mut state.rig;
                    rig.homed = value("homed") == Some("1");
                    rig.position = value("position").and_then(|text| text.parse().ok());
                    rig.first = value("first").filter(|&text| text != "-").map(str::to_owned);
                    rig.span = value("span").and_then(|text| text.parse().ok());
                    if !rig.homed {
                        rig.second = None;
                    }
                }
                Ok(format!("the board answered: {}", reply.values.join(" ")))
            }
            Action::Experiment => experiment::run(self).await,
            // Flashing and the hard reset run in their own handlers
            // (flash.rs, `hard_reset`), never here.
            Action::Flash | Action::HardReset => Ok(String::new()),
        }
    }
}
