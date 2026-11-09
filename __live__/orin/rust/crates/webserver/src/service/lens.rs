//! The lens measurement: `calibrate_service.py` runs the camera, finds the board and
//! keeps the stills; this only passes the page's requests on. It needs no permit,
//! because it only watches; the service keeps it, the live view and a measurement
//! from using the camera together.

use crate::{
    arduino::arduino_state::ArduinoState,
    run::is_running,
    service::{forward, refused},
};

use axum::{extract::State, http::StatusCode, response::Response};

/// Starts the lens view; it stops the live view. Refused while a measurement
/// runs, and while the brain has the camera for a run (`real-loop-dia.md`, *The
/// run and idle states*).
pub(crate) async fn start(State(arduino): State<ArduinoState>) -> Response {
    if is_running(&arduino.run).await {
        return refused(
            StatusCode::CONFLICT,
            "a run is going, and the brain has the camera",
        );
    }
    forward(&arduino.calibration, true, "/lens/start".to_string()).await
}

/// Stops the lens view.
pub(crate) async fn stop(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/stop".to_string()).await
}

/// The latest frame, the box to fill and what was found in it. Asking keeps
/// the view on.
pub(crate) async fn status(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, false, "/lens/status".to_string()).await
}

/// Moves the box on without keeping a still.
pub(crate) async fn skip(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/skip".to_string()).await
}

/// Forgets every kept still.
pub(crate) async fn restart(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/restart".to_string()).await
}

/// Saves the current frame, drawn as the page shows it, for someone not at the
/// tablet. The service writes it into the lens page's folder, so it is served there.
pub(crate) async fn still(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/still".to_string()).await
}

/// Solves the lens from the kept stills, and stores `lens.json`.
pub(crate) async fn solve(State(arduino): State<ArduinoState>) -> Response {
    forward(&arduino.calibration, true, "/lens/solve".to_string()).await
}
