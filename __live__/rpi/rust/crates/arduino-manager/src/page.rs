//! The page, for the rig as it runs today: the firmware drives the rig
//! from its own page over WiFi, and this service flashes and keeps the log.
//! So this page shows the serial line and the log and links to the board's
//! page. It has no controls: no firmware speaks the USB commands yet.

use std::fmt::Write;

use crate::station::Station;

const CSS: &str = "body{font-family:sans-serif;margin:16px;max-width:820px}\
    pre{background:#f3f3f3;padding:10px;overflow-x:auto}\
    .board{font-size:20px;padding:12px 14px;margin:8px 0;border:2px solid #36c;border-radius:8px}";

pub fn render(station: &Station) -> String {
    let state = station.state();
    let settings = &station.settings;
    let mut out = String::new();
    let _ = write!(out, "<!doctype html><html><head><meta charset=\"utf-8\">\
        <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    if state.busy.is_some() {
        out.push_str("<meta http-equiv=\"refresh\" content=\"2\">");
    }
    let _ = write!(out, "<title>Arduino manager</title><style>{CSS}</style></head><body><h1>Arduino manager</h1>");
    let board = escape(&settings.station.board_page);
    let _ = write!(
        out,
        "<div class=\"board\">The rig is run from the board's own page, with its STOP: \
         <a href=\"{board}\">{board}</a></div>"
    );
    if let Some(warning) = &state.warning {
        let freed = if warning.fault { " The driver is off: home the cart again." } else { "" };
        let _ = write!(out, "<div class=\"board\"><b>Halted:</b> {}{freed}</div>", escape(&warning.text));
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
    if let Some(action) = state.busy {
        let _ = writeln!(text, "running        {}", action.label());
    }
    if let Some((action, outcome)) = &state.last {
        let _ = writeln!(text, "last           {}: {outcome}", action.label());
    }
    drop(state);

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

/// Text written into HTML: `&`, `<` and `>` escaped.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
