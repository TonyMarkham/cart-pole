//! The Orin's webserver. The Orin has no monitor, so this is how it shows
//! things: it serves the browser pages, pushes images to the viewer, asks the
//! rig's Arduino for its pins and to move the cart, and passes the
//! measurements on to `calibrate_service.py`. Its settings are in
//! `settings.toml`, read once at the start.

mod arduino;
mod calibrate;
mod constants;
mod error;
mod hello;
mod service;
mod settings;
#[cfg(test)]
mod tests;
mod viewer;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    arduino::arduino_state::ArduinoState,
    constants::{
        ARDUINO_PAGE_DIR, ARDUINO_PAGE_ROUTE, ARDUINO_PINS_ROUTE, ARDUINO_SETTINGS_ROUTE,
        CALIBRATE_CENTRE_ROUTE, CALIBRATE_FIT_ACCEPT_ROUTE, CALIBRATE_FIT_ROUTE,
        CALIBRATE_IDLER_ROUTE, CALIBRATE_MEASURE_ACCEPT_ROUTE, CALIBRATE_MEASURE_CANCEL_ROUTE,
        CALIBRATE_MEASURE_ROUTE, CALIBRATE_MEASURE_STATUS_ROUTE, CALIBRATE_MOTOR_ROUTE,
        CALIBRATE_PAGE_DIR, CALIBRATE_PAGE_ROUTE, CALIBRATE_STATE_ROUTE, CALIBRATE_STOP_ROUTE,
        HELLO_ROUTE, IMAGE_ROUTE, LENS_PAGE_DIR, LENS_PAGE_ROUTE, LENS_RESTART_ROUTE,
        LENS_SKIP_ROUTE, LENS_SOLVE_ROUTE, LENS_START_ROUTE, LENS_STATUS_ROUTE, LENS_STILL_ROUTE,
        LENS_STOP_ROUTE, LIVE_PAGE_DIR, LIVE_PAGE_ROUTE, LIVE_START_ROUTE, LIVE_STATUS_ROUTE,
        LIVE_STOP_ROUTE, SETTINGS_PATH, VIEWER_DIR, VIEWER_ROUTE, WS_ROUTE,
    },
    error::app_result::AppResult,
    service::{lens, live, measure},
    settings::Settings,
};

use axum::{Router, extract::DefaultBodyLimit, routing};
use tokio::{net::TcpListener, sync::watch};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> AppResult<()> {
    let settings = Settings::load(SETTINGS_PATH)?;
    let (images, _) = watch::channel(None);

    let rig = Router::new()
        .route(ARDUINO_PINS_ROUTE, routing::get(arduino::pins))
        .route(ARDUINO_SETTINGS_ROUTE, routing::get(arduino::settings))
        .route(CALIBRATE_IDLER_ROUTE, routing::post(calibrate::idler))
        .route(CALIBRATE_MOTOR_ROUTE, routing::post(calibrate::motor))
        .route(CALIBRATE_CENTRE_ROUTE, routing::post(calibrate::centre))
        .route(CALIBRATE_STOP_ROUTE, routing::post(calibrate::stop))
        .route(CALIBRATE_STATE_ROUTE, routing::get(measure::state))
        .route(CALIBRATE_MEASURE_ROUTE, routing::post(measure::start))
        .route(
            CALIBRATE_MEASURE_STATUS_ROUTE,
            routing::get(measure::status),
        )
        .route(
            CALIBRATE_MEASURE_CANCEL_ROUTE,
            routing::post(measure::cancel),
        )
        .route(
            CALIBRATE_MEASURE_ACCEPT_ROUTE,
            routing::post(measure::accept),
        )
        .route(CALIBRATE_FIT_ROUTE, routing::post(measure::fit))
        .route(
            CALIBRATE_FIT_ACCEPT_ROUTE,
            routing::post(measure::accept_fit),
        )
        .route(LIVE_START_ROUTE, routing::post(live::start))
        .route(LIVE_STOP_ROUTE, routing::post(live::stop))
        .route(LIVE_STATUS_ROUTE, routing::get(live::status))
        .route(LENS_START_ROUTE, routing::post(lens::start))
        .route(LENS_STOP_ROUTE, routing::post(lens::stop))
        .route(LENS_STATUS_ROUTE, routing::get(lens::status))
        .route(LENS_SKIP_ROUTE, routing::post(lens::skip))
        .route(LENS_RESTART_ROUTE, routing::post(lens::restart))
        .route(LENS_SOLVE_ROUTE, routing::post(lens::solve))
        .route(LENS_STILL_ROUTE, routing::post(lens::still))
        .nest_service(ARDUINO_PAGE_ROUTE, ServeDir::new(ARDUINO_PAGE_DIR))
        .nest_service(CALIBRATE_PAGE_ROUTE, ServeDir::new(CALIBRATE_PAGE_DIR))
        .nest_service(LIVE_PAGE_ROUTE, ServeDir::new(LIVE_PAGE_DIR))
        .nest_service(LENS_PAGE_ROUTE, ServeDir::new(LENS_PAGE_DIR))
        .with_state(ArduinoState::new(settings.arduino, settings.calibration));

    let app = Router::new()
        .route(HELLO_ROUTE, routing::get(hello::hello))
        .route(
            IMAGE_ROUTE,
            routing::post(viewer::images::push_image)
                .layer(DefaultBodyLimit::max(settings.server.max_image_bytes)),
        )
        .route(WS_ROUTE, routing::get(viewer::ws::ws_handler))
        .nest_service(VIEWER_ROUTE, ServeDir::new(VIEWER_DIR))
        .with_state(images)
        .merge(rig);

    let listener = TcpListener::bind(&settings.server.address).await?;
    println!("Server started at http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
