mod arduino;
mod constants;
mod error;
mod images;
mod pages;
mod settings;
mod ws;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use crate::constants::{
    ADDRESS, ARDUINO_PAGE_DIR, ARDUINO_PAGE_ROUTE, ARDUINO_PINS_ROUTE, ARDUINO_SETTINGS_ROUTE,
    HELLO_ROUTE, IMAGE_ROUTE, MAX_IMAGE_BYTES, SETTINGS_PATH, VIEWER_DIR, VIEWER_ROUTE, WS_ROUTE,
};

// ---------------------------------------------------------------------------------------------- //

use crate::{
    arduino::{ArduinoState, pins, settings as arduino_settings},
    error::result::AppResult,
    images::push_image,
    pages::hello::hello,
    settings::Settings,
    ws::ws_handler,
};

use axum::{Router, extract::DefaultBodyLimit, routing};
use tokio::{net::TcpListener, sync::watch};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> AppResult<()> {
    let settings = Settings::load(SETTINGS_PATH)?;
    let (images, _) = watch::channel(None);

    let arduino = Router::new()
        .route(ARDUINO_PINS_ROUTE, routing::get(pins))
        .route(ARDUINO_SETTINGS_ROUTE, routing::get(arduino_settings))
        .nest_service(ARDUINO_PAGE_ROUTE, ServeDir::new(ARDUINO_PAGE_DIR))
        .with_state(ArduinoState::new(settings.arduino));

    let app = Router::new()
        .route(HELLO_ROUTE, routing::get(hello))
        .route(
            IMAGE_ROUTE,
            routing::post(push_image).layer(DefaultBodyLimit::max(MAX_IMAGE_BYTES)),
        )
        .route(WS_ROUTE, routing::get(ws_handler))
        .nest_service(VIEWER_ROUTE, ServeDir::new(VIEWER_DIR))
        .with_state(images)
        .merge(arduino);

    let listener = TcpListener::bind(ADDRESS).await?;
    println!("Server started at http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
