//! The live view: `calibrate_service.py` runs the camera and the detector, and this
//! only passes the page's requests on. It needs no permit, because it only watches;
//! the service itself keeps it and a measurement from using the camera together.

use crate::{arduino::arduino_state::ArduinoState, service::forward};

use axum::{extract::State, response::Response};

/// Starts the live view. Refused while a measurement runs.
pub(crate) async fn start(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/live/start".to_string()).await
}

/// Stops the live view.
pub(crate) async fn stop(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/live/stop".to_string()).await
}

/// The latest frame, its boxes and how often each marker was found. Asking
/// keeps the view on.
pub(crate) async fn status(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, false, "/live/status".to_string()).await
}
