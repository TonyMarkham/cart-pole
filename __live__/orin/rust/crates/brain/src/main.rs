//! The Orin's brain: the program on the rig that runs the balance loop. It
//! takes each camera frame, has its Python worker find the markers, keeps the
//! pool, has the worker ask the policy for a command, and sends the command
//! to the Arduino, over and over until the person says stop or the board
//! reports a fault (`real-loop-dia.md`, *The sequence*). It is its own
//! program, a Rust orchestrator, by the user's decision of 2026-11-04
//! (`real-open-issues.md`, issue 3). Its settings are `[brain]` and `[camera]`
//! in `settings.toml`, the file it shares with the webserver, read once at the
//! start.

mod arduino;
mod camera;
mod command;
mod constants;
mod error;
mod geometry;
mod history;
mod inputs;
mod log;
mod pool;
mod run;
mod service;
mod settings;
#[cfg(test)]
mod tests;
mod worker;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{IDLE_ROUTE, RUN_ROUTE, SETTINGS_PATH, STATUS_ROUTE},
    error::app_result::AppResult,
    run::brain::Brain,
    settings::Settings,
    worker::Worker,
};

use axum::{Router, routing};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> AppResult<()> {
    let settings = Settings::load(SETTINGS_PATH)?;

    // The worker loads the TensorRT engine and the policy once, here, so a Run
    // press never waits for them and never fails on them.
    let worker = Worker::start(&settings.brain)?;
    println!(
        "worker ready: policy {}, top rate {} pulses/s, {} past commands, ages in units of {} ms",
        worker.values.source,
        worker.values.top_pulse_rate,
        worker.values.past_commands,
        worker.values.age_unit_ms
    );

    let address = settings.brain.address.clone();
    let brain = Brain::new(settings, worker);
    let app = Router::new()
        .route(RUN_ROUTE, routing::post(service::run))
        .route(IDLE_ROUTE, routing::post(service::idle))
        .route(STATUS_ROUTE, routing::get(service::status))
        .with_state(brain);

    let listener = TcpListener::bind(&address).await?;
    println!("brain idle at http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
