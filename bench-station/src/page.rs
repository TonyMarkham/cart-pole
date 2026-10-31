//! The page, for the rig as it runs today (user, 2026-10-27): the old
//! WiFi firmware drives the rig from its own page, and the Pi only
//! flashes and keeps the trial records. So this page shows the jump-limit
//! run's progress and records, the serial line for flashing, and the log,
//! and links to the board's page. It has no controls: the USB commands
//! they sent are not spoken by the old firmware.

use std::fmt::Write;

use crate::{kind::Kind, settings::Experiment, station::Station, trials::Place};

/// While trial records keep arriving, the page reloads itself this often,
/// s, so the progress bars move; it stops once none has come for
/// `RECORDS_LIVE_S`.
const RECORDS_REFRESH_S: u64 = 10;
const RECORDS_LIVE_S: u64 = 300;

const CSS: &str = "body{font-family:sans-serif;margin:16px;max-width:820px}\
    pre{background:#f3f3f3;padding:10px;overflow-x:auto}\
    .board{font-size:20px;padding:12px 14px;margin:8px 0;border:2px solid #36c;border-radius:8px}\
    .bars{padding:10px 14px;margin:8px 0;border:1px solid #bbb;border-radius:8px}\
    .bars progress{width:100%;height:18px}";

pub fn render(station: &Station) -> String {
    let state = station.state();
    let settings = &station.settings;
    let mut out = String::new();
    let _ = write!(out, "<!doctype html><html><head><meta charset=\"utf-8\">\
        <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    let records_live = station
        .trials
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .last
        .as_ref()
        .is_some_and(|place| place.at.elapsed().as_secs() < RECORDS_LIVE_S);
    if state.busy.is_some() {
        out.push_str("<meta http-equiv=\"refresh\" content=\"2\">");
    } else if records_live {
        let _ = write!(out, "<meta http-equiv=\"refresh\" content=\"{RECORDS_REFRESH_S}\">");
    }
    let _ = write!(out, "<title>cart-pole bench</title><style>{CSS}</style></head><body><h1>cart-pole bench</h1>");
    let board = escape(&settings.station.board_page);
    let _ = write!(
        out,
        "<div class=\"board\">The rig is run from the board's own page, with its STOP: \
         <a href=\"{board}\">{board}</a></div>"
    );

    if let Some(place) = &station.trials.lock().unwrap_or_else(std::sync::PoisonError::into_inner).last {
        progress(&mut out, place, &settings.experiment);
    }

    let mut text = String::new();
    let _ = writeln!(
        text,
        "serial line    {} at {} baud: {}",
        settings.serial.port,
        settings.serial.baud,
        if station.link.connected() { "open" } else { "closed" }
    );
    let _ = writeln!(text, "               (plugged in only to flash: POST the .bin to /flash)");
    if state.busy.is_some() {
        text.push_str("flashing now\n");
    }
    if let Some((action, outcome)) = &state.last {
        let _ = writeln!(text, "last           {}: {outcome}", action.label());
    }
    drop(state);

    {
        let trials = station.trials.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = writeln!(
            text,
            "\ntrials received from the Arduino: {} since the service started ({})",
            trials.count,
            trials.path().display()
        );
        for line in trials.recent() {
            text.push_str(line);
            text.push('\n');
        }
    }

    let log = station.log.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ = writeln!(
        text,
        "\nlog ({}; newest {} lines)",
        log.path().map_or("no file".to_owned(), |path| path.display().to_string()),
        settings.station.page_log_lines
    );
    for line in log.recent() {
        text.push_str(line);
        text.push('\n');
    }
    drop(log);

    let _ = write!(out, "<pre>{}</pre></body></html>", escape(&text));
    out
}

/// The jump-limit run's progress, from the newest trial record the
/// Arduino sent: which of the four kinds, which j step inside it, and
/// which trial at that j. A kind ends early at its first slip, so its j
/// steps are counted out of the most it can take (`Kind::j_steps`).
fn progress(out: &mut String, place: &Place, settings: &Experiment) {
    out.push_str("<div class=\"bars\"><b>Jump-limit run</b>");
    let Some((index, kind)) = Kind::ALL.into_iter().enumerate().find(|(_, kind)| kind.name() == place.kind) else {
        let _ = write!(out, "<p>The last record names a kind the Pi does not know: {}</p></div>", escape(&place.kind));
        return;
    };
    bar(out, &format!("Kind {} of 4: {}", index + 1, kind.name()), index + 1, 4);
    let steps = kind.j_steps(settings);
    match steps.iter().position(|&j| j == place.j) {
        Some(step) => bar(
            out,
            &format!("j step {} of at most {}: j = {} pulses/s²", step + 1, steps.len(), thousands(place.j)),
            step + 1,
            steps.len(),
        ),
        None => {
            let _ = write!(
                out,
                "<p>j = {} is not on the Pi's j sequence: bench-station.toml [experiment] differs from the firmware's \
                 calibrate.toml</p>",
                thousands(place.j)
            );
        }
    }
    let trials = settings.trials_per_j as usize;
    bar(out, &format!("Trial {} of {} at this j", place.trial, trials), place.trial as usize, trials);
    let _ = write!(out, "<p>last record {} s ago</p></div>", place.at.elapsed().as_secs());
}

fn bar(out: &mut String, label: &str, value: usize, max: usize) {
    let _ = write!(out, "<div>{} <progress value=\"{value}\" max=\"{max}\"></progress></div>", escape(label));
}

/// `n` with commas between thousands: 1,210,937.
fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Text written into HTML: `&`, `<` and `>` escaped.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
