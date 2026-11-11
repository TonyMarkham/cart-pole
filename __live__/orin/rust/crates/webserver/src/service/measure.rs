//! The calibration's measurements: each route passes the page's request on to
//! `calibrate_service.py`. A measurement and a move never run together.

use crate::{
    arduino::arduino_state::ArduinoState,
    run::is_running,
    service::{forward, refused},
};

use axum::{
    Json,
    extract::{RawQuery, State},
    http::StatusCode,
    response::Response,
};
use serde_json::{Value, json};

/// Starts a stop's measurement. It holds the moves' permit while it asks, and a
/// move holds that permit while it asks whether a measurement is running, so one
/// can never slip in beside the other. Refused while the brain has the camera
/// for a run (`__documentation__/real-loop-dia.md`, *The run and idle states*).
pub(crate) async fn start(
    State(arduino): State<ArduinoState>,
    RawQuery(query): RawQuery,
) -> Response {
    let Ok(_no_move_meanwhile) = arduino.moving.clone().try_acquire_owned() else {
        return refused(
            StatusCode::CONFLICT,
            "the cart is moving: it can be measured once it has stopped",
        );
    };
    if is_running(&arduino.run).await {
        return refused(
            StatusCode::CONFLICT,
            "a run is going, and the brain has the camera",
        );
    }
    let query = query.unwrap_or_default();
    forward(&arduino.calibration, true, format!("/measure?{query}")).await
}

/// How the measurement under way is going, with its latest frame.
pub(crate) async fn status(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, false, "/status".to_string()).await
}

/// Ends any measurement, and throws its result away.
pub(crate) async fn cancel(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/cancel".to_string()).await
}

/// Keeps the finished measurement for its pose.
pub(crate) async fn accept(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/accept".to_string()).await
}

/// Fits the matrix with the span these moves measured.
pub(crate) async fn fit(State(arduino): State<ArduinoState>) -> Response {
    let span = arduino.rail.lock().await.span;
    let Some(span) = span else {
        return refused(
            StatusCode::CONFLICT,
            "the rail's span is not known: find one end of the rail, then the other",
        );
    };
    forward(&arduino.calibration, true, format!("/fit?span={span}")).await
}

/// Marks the latest fit as the one to use, though a point is over the limit.
pub(crate) async fn accept_fit(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/fit/accept".to_string()).await
}

/// What the moves have found, so a refreshed page knows it too, and how often the
/// page should look at a measurement.
pub(crate) async fn state(State(arduino): State<ArduinoState>) -> Json<Value> {
    let rail = *arduino.rail.lock().await;
    Json(json!({
        "last_stop": rail.last_stop,
        "span": rail.span,
        "poll_ms": arduino.calibration.poll_ms,
    }))
}
