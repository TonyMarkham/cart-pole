//! The page the board serves: the controls, a big STOP, and the run's
//! facts as text. Every button is a form that POSTs to its action's path
//! (action.rs), so loading or reloading the page never moves the cart.

use core::fmt::{self, Write};

use crate::{
    action::Action,
    experiment::ExperimentReport,
    pins::PinStates,
    report::{Report, write_settings},
    settings::JOG_PULSES,
};

const HEAD: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
    <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">";

/// The STOP button's look: as wide as the page and hard to miss.
const STOP_CSS: &str = ".stop{display:block;width:100%;font-size:44px;font-weight:bold;padding:30px;\
    margin:8px 0 18px;background:#c00;color:#fff;border:0;border-radius:14px}";

const PAGE_CSS: &str = "body{font-family:sans-serif;margin:16px;max-width:760px}form{display:inline}\
    button{font-size:18px;padding:10px 16px;margin:4px}pre{background:#f3f3f3;padding:10px;overflow-x:auto}";

fn stop_button(out: &mut impl Write) -> fmt::Result {
    write!(
        out,
        "<form method=\"post\" action=\"{}\" style=\"display:block\"><button class=\"stop\">STOP</button></form>",
        Action::Stop.path()
    )
}

/// The whole page, served when nothing is moving.
pub fn full(out: &mut impl Write, report: &Report, pins: PinStates, log: &str, log_full: bool) -> fmt::Result {
    write!(
        out,
        "{HEAD}<title>cart-pole calibrate</title><style>{PAGE_CSS}{STOP_CSS}</style></head>\
         <body><h1>cart-pole calibrate</h1>"
    )?;
    stop_button(out)?;
    for action in Action::ALL.into_iter().filter(|&action| action != Action::Stop && action != Action::Experiment) {
        write!(
            out,
            "<form method=\"post\" action=\"{}\"><button>{}</button></form>",
            action.path(),
            action.label()
        )?;
    }
    write!(
        out,
        "<p>After a button, the page waits until the action has finished, then shows the result; \
         STOP works while it waits. A jog moves {JOG_PULSES} pulses at the seek rate and stops at \
         either switch. Go to centre needs a finished calibration.</p>"
    )?;
    write!(
        out,
        "<p><form method=\"post\" action=\"{}\"><button>{}</button></form> \
         Measures m for trainer/gym.toml: needs a calibration, takes hours, and STOP ends it. \
         The page answers at once; reload it to follow the trials.</p><pre>",
        Action::Experiment.path(),
        Action::Experiment.label()
    )?;
    let mut text = Escaped(out);
    report.write(&mut text)?;
    writeln!(text)?;
    report.experiment.write(&mut text)?;
    write!(text, "\npin states (1 = high, 0 = low)\n")?;
    pins.report(&mut text)?;
    write!(text, "\nsettings in this build\n")?;
    write_settings(&mut text)?;
    write!(text, "\nserial log since power-up\n{log}")?;
    if log_full {
        writeln!(text, "(log full: later lines went to the serial port only)")?;
    }
    write!(out, "</pre></body></html>")
}

/// The page served to a reload during a move: STOP alone, and the
/// experiment's progress while it runs. Kept short, since sending it
/// pauses the move; it is only sent during a slow move, never during a
/// trial's jump.
pub fn moving(out: &mut impl Write, action: Action, progress: Option<&ExperimentReport>) -> fmt::Result {
    write!(out, "{HEAD}<title>cart-pole: moving</title><style>{STOP_CSS}</style></head><body>")?;
    stop_button(out)?;
    write!(
        out,
        "<p>Moving: {}. Reload once it has ended to see the result and the controls.</p>",
        action.label()
    )?;
    if let Some(progress) = progress {
        write!(out, "<pre>")?;
        progress.write(&mut Escaped(out))?;
        write!(out, "</pre>")?;
    }
    write!(out, "</body></html>")
}

/// Text written into HTML: `&`, `<` and `>` escaped.
struct Escaped<'a, W: Write>(&'a mut W);

impl<W: Write> Write for Escaped<'_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for character in text.chars() {
            match character {
                '&' => self.0.write_str("&amp;")?,
                '<' => self.0.write_str("&lt;")?,
                '>' => self.0.write_str("&gt;")?,
                _ => self.0.write_char(character)?,
            }
        }
        Ok(())
    }
}
