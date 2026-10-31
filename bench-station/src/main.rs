//! The bench station: a service on the Raspberry Pi beside the rig
//! (bench-station-plan.md). The Arduino keeps only the real-time work
//! (the pulses, the switches, the commands); this service does the rest
//! over the Arduino's USB serial line: the web page and its buttons, the
//! jump-limit experiment, a log file of every line, and flashing the
//! `.bin` the PC built.
//!
//! - Settings: bench-station.toml (the path is the first argument).
//! - The protocol: protocol.rs, and README.md, *The protocol*.
//! - Build, run and flash: README.md.

mod experiment;
mod flash;
mod halt;
mod kind;
mod link;
mod log;
mod page;
mod profile;
mod protocol;
mod settings;
mod station;
mod trials;
#[cfg(test)]
mod tests;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use clap::Parser;

use crate::{
    link::Link,
    log::LogBook,
    settings::Settings,
    station::{Action, Station},
};

#[derive(Parser)]
#[command(about = "The bench station: the rig's page, experiment, log and flashing, on the Pi")]
struct Args {
    /// The settings file.
    #[arg(default_value = "bench-station.toml")]
    settings: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let settings = Settings::load(&args.settings)?;
    let log = Arc::new(Mutex::new(LogBook::new(&settings.station.log_dir, settings.station.page_log_lines)));
    let link = Link::start(&settings.serial, Arc::clone(&log));
    let listen = settings.station.listen.clone();
    let station = Arc::new(Station::new(settings, link, log));
    station.watch_resets();

    let mut app = Router::new()
        .route("/", get(page))
        .route("/stop", post(stop))
        .route("/reset", post(reset))
        .route(Action::HardReset.path(), post(hard_reset))
        .route("/trial", post(trial))
        .route(Action::Flash.path(), post(flash).layer(DefaultBodyLimit::max(flash::MAX_BIN)));
    for action in Action::BUTTONS {
        app = app.route(action.path(), post(move |state| button(state, action)));
    }
    let app = app.with_state(Arc::clone(&station));

    let listener = tokio::net::TcpListener::bind(&listen).await.with_context(|| format!("listening on {listen}"))?;
    station.note(&format!("bench station listening on http://{listen}/"));
    axum::serve(listener, app).await.context("serving the page")?;
    Ok(())
}

async fn page(State(station): State<Arc<Station>>) -> Html<String> {
    Html(page::render(&station))
}

async fn stop(State(station): State<Arc<Station>>) -> Redirect {
    station.stop().await;
    Redirect::to("/")
}

async fn reset(State(station): State<Arc<Station>>) -> Redirect {
    if !station.reset() {
        station.note("Reset ignored: an action is running; press STOP first");
    }
    Redirect::to("/")
}

async fn hard_reset(State(station): State<Arc<Station>>) -> Redirect {
    station.hard_reset().await;
    Redirect::to("/")
}

/// Starts a button's action. A press while another runs is ignored, as
/// on calibrate's page; the page shows what is running.
async fn button(State(station): State<Arc<Station>>, action: Action) -> Redirect {
    if !station.start(action) {
        station.note(&format!("{} ignored: another action is running", action.label()));
    }
    Redirect::to("/")
}

/// One trial record from the Arduino (trials.rs): `ok`, or what was
/// wrong with it.
async fn trial(State(station): State<Arc<Station>>, body: String) -> (StatusCode, String) {
    let recorded = station.trials.lock().unwrap_or_else(std::sync::PoisonError::into_inner).record(&body);
    match recorded {
        Ok(()) => (StatusCode::OK, "ok\n".to_owned()),
        Err(why) => {
            station.note(&format!("trial record refused ({why}): {}", body.trim()));
            (StatusCode::BAD_REQUEST, format!("{why}\n"))
        }
    }
}

/// The body is the `.bin`; the answer is `arduino-cli`'s output.
async fn flash(State(station): State<Arc<Station>>, image: Bytes) -> Response {
    let (ok, output) = flash::flash(&station, &image).await;
    let status = if ok { StatusCode::OK } else { StatusCode::INTERNAL_SERVER_ERROR };
    (status, output).into_response()
}
