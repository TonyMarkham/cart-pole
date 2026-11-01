use crate::{arduino::ArduinoState, settings::CalibrationSettings};

use axum::{
    Json,
    extract::{RawQuery, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::time::Duration;

/// Starts a stop's measurement. It holds the moves' permit while it asks, and a
/// move holds that permit while it asks whether a measurement is running, so one
/// can never slip in beside the other.
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
    let query = query.unwrap_or_default();
    forward(&arduino.calibration, true, format!("/measure?{query}")).await
}

pub(crate) async fn status(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, false, "/status".to_string()).await
}

pub(crate) async fn cancel(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/cancel".to_string()).await
}

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

/// Whether a measurement is running. A service that is not running is not
/// measuring, and the moves do not depend on it.
pub(crate) async fn is_measuring(calibration: &CalibrationSettings) -> bool {
    match ask(calibration, false, "/busy".to_string()).await {
        Ok((_, body)) => serde_json::from_slice::<Value>(&body)
            .is_ok_and(|answer| answer["busy"].as_bool() == Some(true)),
        Err(_) => false,
    }
}

async fn forward(calibration: &CalibrationSettings, post: bool, path: String) -> Response {
    match ask(calibration, post, path).await {
        Ok((status, body)) => {
            (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
        }
        Err(reason) => refused(
            StatusCode::BAD_GATEWAY,
            &format!("the calibration service did not answer, so it may not be running: {reason}"),
        ),
    }
}

fn refused(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

/// One request to `calibrate_service.py`, on a thread of its own because the HTTP
/// client blocks. Its 4xx answers are answers, not failures.
async fn ask(
    calibration: &CalibrationSettings,
    post: bool,
    path: String,
) -> Result<(StatusCode, Vec<u8>), String> {
    let url = format!("{}{path}", calibration.service);
    let timeout = Duration::from_millis(calibration.timeout_ms);
    tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .into();
        let sent = if post {
            agent.post(&url).send_empty()
        } else {
            agent.get(&url).call()
        };
        let mut response = sent.map_err(|error| error.to_string())?;
        let status =
            StatusCode::from_u16(response.status().as_u16()).map_err(|error| error.to_string())?;
        let body = response
            .body_mut()
            .read_to_vec()
            .map_err(|error| error.to_string())?;
        Ok((status, body))
    })
    .await
    .map_err(|error| error.to_string())?
}
