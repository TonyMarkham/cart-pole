//! The live view: `calibrate_service.py` runs the camera and the detector, and this
//! only passes the page's requests on. It needs no permit, because it only watches;
//! the service itself keeps it and a measurement from using the camera together.

use crate::{
    arduino::arduino_state::ArduinoState,
    run::is_running,
    service::{forward, refused},
};

use axum::{extract::State, http::StatusCode, response::Response};

/// Starts the live view. Refused while a measurement runs, and while the brain
/// has the camera for a run (`real-loop-dia.md`, *The run and idle states*).
pub(crate) async fn start(State(arduino): State<ArduinoState>) -> Response {
    if is_running(&arduino.run).await {
        return refused(
            StatusCode::CONFLICT,
            "a run is going, and the brain has the camera",
        );
    }
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
