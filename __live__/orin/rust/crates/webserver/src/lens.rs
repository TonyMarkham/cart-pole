//! The lens measurement: `calibrate_service.py` runs the camera, finds the board and
//! keeps the stills; this only passes the page's requests on. It needs no permit,
//! because it only watches; the service keeps it, the live view and a measurement
//! from using the camera together.

use crate::{arduino::ArduinoState, measure::forward};

use axum::{extract::State, response::Response};

pub(crate) async fn start(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/start".to_string()).await
}

pub(crate) async fn stop(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/stop".to_string()).await
}

pub(crate) async fn status(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, false, "/lens/status".to_string()).await
}

pub(crate) async fn skip(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/skip".to_string()).await
}

pub(crate) async fn restart(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/restart".to_string()).await
}

/// Saves the current frame, drawn as the page shows it, for someone not at the
/// tablet. The service writes it into the lens page's folder, so it is served there.
pub(crate) async fn still(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/still".to_string()).await
}

pub(crate) async fn solve(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/solve".to_string()).await
}
